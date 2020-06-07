#![macro_use]

use std::cell::Cell;
use std::collections::BTreeMap;

use super::fields::{Convertible, Item, ItemBuilder, Or};
use super::random::Rand;

pub struct RuleSet {
    pub rule_map: BTreeMap<String, usize>,
    pub rule_map_inv: BTreeMap<usize, String>,
    pub rules: Vec<Or>,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        RuleSet {
            rule_map: BTreeMap::new(),
            rule_map_inv: BTreeMap::new(),
            rules: Vec::new(),
        }
    }

    pub fn add_rule<T, S>(&mut self, rule_name: S, rule_value: T) -> &mut Self
    where
        S: Into<String>,
        T: Convertible,
    {
        let rule_name = rule_name.into();

        let rule_idx = match self.rule_map.get(&rule_name) {
            None => {
                let res = self.rules.len();
                self.rule_map.insert(rule_name.clone(), res);
                self.rule_map_inv.insert(res, rule_name);
                self.rules.push(Or::new());
                res
            }
            Some(v) => *v,
        };
        self.rules[rule_idx].add_item(rule_value);
        self
    }

    pub fn finalize(&mut self) {
        loop {
            let mut num_pruned: usize = 0;
            num_pruned += self.finalize_and_prune_rules();
            println!("finalize pruned {}", num_pruned);
            num_pruned += self.calc_shortest_ref_length();
            println!("shortest ref pruned {}", num_pruned);
            if num_pruned == 0 {
                break;
            }
        }
    }

    pub fn finalize_and_prune_rules(&mut self) -> usize {
        println!("Finalizing and pruning rules");
        let mut total_pruned = 0;
        loop {
            let mut num_pruned = 0;
            for (rule_idx, rule_or) in self.rules.iter_mut().enumerate() {
                let rule_name = &self.rule_map_inv[&rule_idx];
                // has already been pruned
                if !self.rule_map.contains_key(rule_name) {
                    continue;
                }

                let fetcher = RefFetcher {
                    rule_map: &self.rule_map,
                };
                // rule Or has no options left, everything is unresolvable
                if !rule_or.finalize(&fetcher) {
                    println!("Pruning rule {} due to unresolvable references", rule_name);
                    self.rule_map.remove(rule_name);
                    total_pruned += 1;
                    num_pruned += 1;
                }
            }
            if num_pruned == 0 {
                break;
            }
        }
        total_pruned
    }

    pub fn calc_shortest_ref_length(&mut self) -> usize {
        println!("Calculating shortest ref lengths");
        let mut rule_lengths: BTreeMap<usize, usize> = BTreeMap::new();
        let mut total_pruned = 0;

        loop {
            let mut num_resolved: usize = 0;
            // we only iterate over the rules with resolvable references
            for (_, rule_idx) in self.rule_map.iter() {
                let rule_or = self.rules.get_mut(*rule_idx).unwrap();

                let num_options_before = rule_or.shortest_options.len();
                let length_calc = RefLenCalculator {
                    rule_lengths: &rule_lengths,
                };
                let new_len = rule_or.calc_ref_length(&length_calc);
                if new_len != 0 {
                    rule_lengths.insert(*rule_idx, new_len);
                }
                let num_options = rule_or.shortest_options.len();
                if num_options > num_options_before {
                    println!(
                        "Resolved {} new options for {}, total {}",
                        num_options - num_options_before,
                        self.rule_map_inv[rule_idx],
                        num_options
                    );
                    num_resolved += 1;
                }
            }
            // there was nothing new that was resolved
            if num_resolved == 0 {
                break;
            }
        }

        for (rule_idx, _) in self.rules.iter().enumerate() {
            if rule_lengths.contains_key(&rule_idx)
                || !self.rule_map.contains_key(&self.rule_map_inv[&rule_idx])
            {
                continue;
            }
            println!(
                "Pruning rule {} due to undeterminable reference length",
                self.rule_map_inv[&rule_idx]
            );
            self.rule_map.remove(&self.rule_map_inv[&rule_idx]);
            total_pruned += 1;
        }

        total_pruned
    }

    pub fn get_ref_idx<T>(&self, rule_name: T) -> Option<usize>
    where
        T: Into<String>,
    {
        let rule_name = rule_name.into();
        Some(*self.rule_map.get(&rule_name)?)
    }

    /// Build the rule specified by ref_idx, with output added to `output`,
    /// using `rand`, and the maximum recursion depth of `max_recursion`.
    pub fn build_rule(
        &self,
        ref_idx: usize,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        max_recursion: usize,
    ) {
        let builder = ItemBuilder {
            rules: &self.rules,
            curr_depth: Cell::new(0),
            max_depth: max_recursion,
        };
        builder.build_rule(ref_idx, output, rand, false);
    }

    #[allow(dead_code)]
    pub fn get_rule_slow<'a, T>(&'a self, rule_name: T, rand: &mut Rand) -> Option<&'a Item>
    where
        T: Into<String>,
    {
        let ref_idx = self.get_ref_idx(rule_name)?;
        let rule_or: &Or = self.rules.get(ref_idx)?;
        Some(rule_or.get_item(rand, false))
    }

    #[allow(dead_code)]
    pub fn build_rule_slow<'a, T>(
        &'a self,
        rule_name: T,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        max_recursion: usize,
    ) where
        T: Into<String>,
    {
        let builder = ItemBuilder {
            rules: &self.rules,
            curr_depth: Cell::new(0),
            max_depth: max_recursion,
        };

        let rule = self
            .get_rule_slow(rule_name, rand)
            .expect("Rule does not exist!");
        builder.build(rule, output, rand);
    }
}

pub struct RefLenCalculator<'a> {
    rule_lengths: &'a BTreeMap<usize, usize>,
}

impl<'a> RefLenCalculator<'a> {
    pub fn calc_ref_length(&'a self, item: &mut Item) -> usize {
        match item {
            Item::And(v) => v.calc_ref_length(self),
            Item::Or(v) => v.calc_ref_length(self),
            Item::Ref(v) => v.calc_ref_length(self),
            Item::Opt(v) => v.calc_ref_length(self),
            Item::Mul(v) => v.calc_ref_length(self),
            _ => 1,
        }
    }

    pub fn get_ref_len(&'a self, rule_idx: usize) -> Option<usize> {
        match self.rule_lengths.get(&rule_idx) {
            Some(v) => Some(*v),
            None => None,
        }
    }
}

pub struct RefFetcher<'a> {
    pub rule_map: &'a BTreeMap<String, usize>,
}

impl<'a> RefFetcher<'a> {
    /// Finalize the `Item`, returning true if the item is fully resolvable
    pub fn finalize(&self, item: &mut Item) -> bool {
        match item {
            Item::And(v) => v.finalize(&self),
            Item::Ref(v) => v.finalize(&self),
            Item::Or(v) => v.finalize(&self),
            Item::Opt(v) => v.finalize(&self),
            Item::Mul(v) => v.finalize(&self),
            Item::Direct(_) => true,
            Item::Str(_) => true,
            Item::Int(_) => true,
        }
    }

    pub fn get_ref_idx<T>(&'a self, rule_name: T) -> Option<usize>
    where
        T: Into<String>,
    {
        let rule_name = rule_name.into();
        let rule_idx = *self.rule_map.get(&rule_name)?;
        Some(rule_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Rand;
    use std::str;

    #[test]
    fn test_rule_set() {
        let mut rules = RuleSet::new();
        let mut rand = Rand::new(0);
        let rules = rules.add_rule("rule", and!(sep = "", "hello", "there"));
        rules.finalize();

        assert_eq!(rules.rules.len(), 1);

        let rule = rules.get_rule_slow("rule", &mut rand);
        assert_eq!(rule.is_some(), true);
    }

    #[test]
    fn test_rule_build() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("rule", and!("hello", "there"))
            .add_rule("rule2", and!("oogah", reff!("rule"), "boogah"));
        let mut rand = Rand::new(0);
        rules.finalize();

        let mut output: Vec<u8> = Vec::new();
        rules.build_rule_slow("rule2", &mut output, &mut rand, 10);
        assert_eq!(
            str::from_utf8(&output[..]).unwrap(),
            "oogahhellothereboogah"
        );
    }

    #[test]
    fn test_auto_prune() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("prune_me", reff!("unresolvable"))
            .add_rule("prune_me2", reff!("prune_me"))
            .add_rule("prune_me3", reff!("prune_me2"))
            .add_rule("rule", "a valid rule")
            .add_rule("rule2", and!("oogah", reff!("rule"), "boogah"));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 2);
        assert_eq!(rules.rule_map.contains_key("rule"), true);
        assert_eq!(rules.rule_map.contains_key("rule2"), true);
        assert_eq!(rules.rule_map.contains_key("prune_me"), false);
        assert_eq!(rules.rule_map.contains_key("prune_me2"), false);
        assert_eq!(rules.rule_map.contains_key("prune_me3"), false);
    }

    #[test]
    fn test_auto_prune_circular() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("prune_me2", reff!("prune_me3"))
            .add_rule("prune_me3", reff!("prune_me2"))
            .add_rule("rule", or!(reff!("prune_me2"), "a valid rule"))
            .add_rule("rule2", and!("oogah", reff!("rule"), "boogah"));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 2);
        assert_eq!(rules.rule_map.contains_key("rule"), true);
        assert_eq!(rules.rule_map.contains_key("rule2"), true);
        assert_eq!(rules.rule_map.contains_key("prune_me2"), false);
        assert_eq!(rules.rule_map.contains_key("prune_me3"), false);
    }

    #[test]
    fn test_auto_prune_circular_or() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("prune_me2", or!(reff!("prune_me3")))
            .add_rule("prune_me3", or!(reff!("prune_me2")))
            .add_rule("rule", or!(reff!("prune_me2"), "a valid rule"))
            .add_rule("rule2", and!("oogah", reff!("rule"), "boogah"));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 2);
        assert_eq!(rules.rule_map.contains_key("rule"), true);
        assert_eq!(rules.rule_map.contains_key("rule2"), true);
        assert_eq!(rules.rule_map.contains_key("prune_me2"), false);
        assert_eq!(rules.rule_map.contains_key("prune_me3"), false);
    }

    #[test]
    fn test_ref_length() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("rule", and!("rule", reff!("rule1")))
            .add_rule("rule1", and!("rule1", or!("short", reff!("rule2"))))
            .add_rule("rule2", and!("rule2", or!("short", reff!("rule3"))))
            .add_rule("rule3", and!("rule3", or!("short", reff!("rule1"))));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 4);

        let ref_idx = rules.get_ref_idx("rule").unwrap();
        let mut rand = Rand::new(11111);

        let mut max_recursion = 1;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_ne!(res, "rulerule");
        }

        max_recursion = 1;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_eq!(["rulerule1short"].contains(&res), true);
        }

        max_recursion = 2;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                ["rulerule1short", "rulerule1rule2short"].contains(&res),
                true
            );
        }

        max_recursion = 3;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                [
                    "rulerule1short",
                    "rulerule1rule2short",
                    "rulerule1rule2rule3short"
                ]
                .contains(&res),
                true
            );
        }

        max_recursion = 4;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                [
                    "rulerule1short",
                    "rulerule1rule2short",
                    "rulerule1rule2rule3short",
                    "rulerule1rule2rule3rule1short"
                ]
                .contains(&res),
                true
            );
        }
    }
}
