#![macro_use]

use std::cell::Cell;
use std::collections::BTreeMap;

use super::fields::{Convertible, Item, ItemBuilder};
use super::random::Rand;

pub struct RuleSet {
    pub rule_map: BTreeMap<String, usize>,
    pub rule_map_inv: BTreeMap<usize, String>,
    pub rules: Vec<Vec<(Item, usize)>>,
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
                self.rules.push(Vec::new());
                res
            }
            Some(v) => *v,
        };
        let converted = rule_value.convert();
        self.rules[rule_idx].push(converted); //rule_value.convert());
        self
    }

    pub fn finalize(&mut self) {
        let mut to_prune: Vec<(usize, usize)> = Vec::new();
        loop {
            let fetcher = RefFetcher {
                rule_map: &self.rule_map,
            };

            for (rule_idx, rule_list) in self.rules.iter_mut().enumerate() {
                for (opt_idx, rule_opt) in rule_list.iter_mut().enumerate() {
                    if !fetcher.finalize(rule_opt) {
                        println!(
                            "Could not finalize {}[{}]: {}",
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
                self.rules[*rule_idx].remove(*opt_idx);
                // keep the list, but remove it from the rule_map to make
                // referencing refs fail
                if self.rules[*rule_idx].len() == 0 {
                    self.rule_map
                        .remove(self.rule_map_inv.get(rule_idx).unwrap());
                }
            }

            to_prune.clear();
        }

        self.calc_shortest_ref_length();
    }

    fn calc_shortest_ref_length(&mut self) {}

    pub fn get_ref_idx<T>(&self, rule_name: T) -> Option<usize>
    where
        T: Into<String>,
    {
        let rule_name = rule_name.into();
        Some(*self.rule_map.get(&rule_name)?)
    }

    pub fn build_rule(&self, ref_idx: usize, output: &mut Vec<u8>, rand: &mut Rand) {
        let builder = ItemBuilder {
            rules: &self.rules,
            curr_depth: Cell::new(0),
            max_depth: 10,
        };

        let rule = builder.fetch_rule(ref_idx, rand).unwrap();

        builder.build(rule, output, rand);
    }

    #[allow(dead_code)]
    pub fn get_rule_slow<'a, T>(&'a self, rule_name: T, rand: &mut Rand) -> Option<&'a Item>
    where
        T: Into<String>,
    {
        let ref_idx = self.get_ref_idx(rule_name)?;
        let rules = self.rules.get(ref_idx)?;
        let rand_idx = rand.rand_u64(0, rules.len() as u64) as usize;
        let res = rules.get(rand_idx)?;
        Some(res)
    }

    #[allow(dead_code)]
    pub fn build_rule_slow<'a, T>(&'a self, rule_name: T, output: &mut Vec<u8>, rand: &mut Rand)
    where
        T: Into<String>,
    {
        let builder = ItemBuilder {
            rules: &self.rules,
            curr_depth: Cell::new(0),
            max_depth: 10,
        };

        let rule = self
            .get_rule_slow(rule_name, rand)
            .expect("Rule does not exist!");
        builder.build(rule, output, rand);
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
        rules.build_rule_slow("rule2", &mut output, &mut rand);
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
}
