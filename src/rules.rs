#![macro_use]

use std::cell::Cell;
use std::collections::BTreeMap;

use super::fields::{Convertible, Item, ItemBuilder};
use super::random::Rand;

pub struct RuleSet {
    pub rule_map: BTreeMap<String, usize>,
    pub rule_map_inv: BTreeMap<usize, String>,
    pub rules: Vec<(Vec<(Item, usize)>, Vec<usize>)>,
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
                self.rules.push((Vec::new(), Vec::new()));
                res
            }
            Some(v) => *v,
        };
        let converted = rule_value.convert();
        self.rules[rule_idx].0.push((converted, 0));
        self
    }

    pub fn finalize(&mut self) {
        self.finalize_and_prune_rules();
        self.calc_shortest_ref_length();
        self.finalize_and_prune_rules();
    }

    pub fn finalize_and_prune_rules(&mut self) {
        println!("Finalizing and pruning rules");
        let mut to_prune: Vec<(usize, usize)> = Vec::new();
        loop {
            let fetcher = RefFetcher {
                rule_map: &self.rule_map,
            };

            for (rule_idx, (rule_list, _)) in self.rules.iter_mut().enumerate() {
                for (opt_idx, (rule_opt, _)) in rule_list.iter_mut().enumerate() {
                    if !fetcher.finalize(rule_opt) {
                        println!(
                            "Pruning {}[{}]: {}",
                            self.rule_map_inv[&rule_idx], opt_idx, rule_opt,
                        );
                        to_prune.push((rule_idx, opt_idx));
                    }
                }
            }

            if to_prune.len() == 0 {
                break;
            }

            // iterate backwards so we remove items from the end of vecs first
            for (rule_idx, opt_idx) in to_prune.iter().rev() {
                self.rules[*rule_idx].0.remove(*opt_idx);
                // keep the list, but remove it from the rule_map to make
                // referencing refs fail
                if self.rules[*rule_idx].0.len() == 0 {
                    self.rule_map
                        .remove(self.rule_map_inv.get(rule_idx).unwrap());
                }
            }

            to_prune.clear();
        }
    }

    pub fn calc_shortest_ref_length(&mut self) {
        println!("Calculating shortest ref lengths");
        let mut rule_lengths: BTreeMap<usize, usize> = BTreeMap::new();

        loop {
            let mut num_resolved: usize = 0;
            for (rule_idx, (rule_list, _)) in self.rules.iter_mut().enumerate() {
                if rule_lengths.contains_key(&rule_idx) || rule_list.len() == 0 {
                    continue;
                }

                let mut min_ref_len = std::usize::MAX;
                for (rule_opt, opt_ref_len) in rule_list.iter_mut() {
                    let length_calc = RefLenCalculator {
                        rule_lengths: &rule_lengths,
                    };

                    let new_len = length_calc.calc_ref_length(rule_opt);
                    if new_len != 0 {
                        if new_len < min_ref_len {
                            min_ref_len = new_len;
                        }
                        if new_len != *opt_ref_len {
                            *opt_ref_len = new_len;
                            println!(
                                "Resolved {} to {}: {}",
                                self.rule_map_inv[&rule_idx], opt_ref_len, rule_opt
                            );
                            num_resolved += 1;
                        }
                    }
                }
                if min_ref_len != std::usize::MAX {
                    rule_lengths.insert(rule_idx, min_ref_len);
                }
            }
            // there was nothing new that was resolved
            if num_resolved == 0 {
                break;
            }
        }

        let mut to_prune: Vec<(usize, usize)> = Vec::new();
        for (rule_idx, (rule_list, rule_shortest)) in self.rules.iter_mut().enumerate() {
            if !rule_lengths.contains_key(&rule_idx) {
                println!(
                    "Pruning rule {} due to undeterminable reference length",
                    self.rule_map_inv[&rule_idx]
                );
                self.rule_map.remove(&self.rule_map_inv[&rule_idx]);
                continue;
            }
            let min_length = rule_lengths[&rule_idx];
            for (rule_opt_idx, (_, opt_ref_len)) in rule_list.iter_mut().enumerate() {
                if *opt_ref_len > 0 {
                    if *opt_ref_len == min_length {
                        rule_shortest.push(rule_opt_idx);
                    }
                    continue;
                }
                to_prune.push((rule_idx, rule_opt_idx));
            }
        }

        for (rule_idx, rule_opt_idx) in to_prune.iter().rev() {
            self.rules[*rule_idx].0.remove(*rule_opt_idx);
            if self.rules[*rule_idx].0.len() == 0 {
                println!(
                    "Pruning rule {} due to undeterminable reference length",
                    self.rule_map_inv[rule_idx]
                );
                self.rule_map.remove(&self.rule_map_inv[rule_idx]);
            }
        }
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

        let rule = builder.fetch_rule(ref_idx, rand, false).unwrap();

        builder.build(rule, output, rand);
    }

    #[allow(dead_code)]
    pub fn get_rule_slow<'a, T>(&'a self, rule_name: T, rand: &mut Rand) -> Option<&'a Item>
    where
        T: Into<String>,
    {
        let ref_idx = self.get_ref_idx(rule_name)?;
        let (rules, _) = self.rules.get(ref_idx)?;
        let rand_idx = rand.rand_u64(0, rules.len() as u64) as usize;
        let (res, _) = rules.get(rand_idx)?;
        Some(res)
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

        let get_rule_len = |name| rules.rules[rules.rule_map[name]].0[0].1;

        assert_eq!(rules.rule_map.len(), 4);
        assert_eq!(get_rule_len("rule"), 2);
        assert_eq!(get_rule_len("rule1"), 1);
        assert_eq!(get_rule_len("rule2"), 1);
        assert_eq!(get_rule_len("rule3"), 1);

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
