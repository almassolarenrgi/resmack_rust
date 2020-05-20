#![macro_use]

use std::collections::BTreeMap;

use crate::fields::{Convertible, Item};

pub struct RuleSet<'a> {
    cat_map: BTreeMap<&'a str, usize>,
    rule_map: BTreeMap<&'a str, usize>,
    pub categories: Vec<Vec<Vec<Item>>>,
    curr_cat: &'a str,
}

impl<'a> RuleSet<'a> {
    pub fn new() -> RuleSet<'a> {
        RuleSet {
            cat_map: BTreeMap::new(),
            rule_map: BTreeMap::new(),
            categories: Vec::new(),
            curr_cat: "",
        }
    }

    pub fn set_category(mut self, cat: &'a str) -> Self {
        self.curr_cat = cat.clone();
        self
    }

    pub fn add_rule<T: Convertible>(mut self, rule_name: &'a str, rule_value: T) -> Self {
        let cat_idx = match self.cat_map.get(self.curr_cat) {
            None => {
                let res = self.categories.len();
                self.cat_map.insert(self.curr_cat, res);
                self.categories.push(Vec::new());
                res
            }
            Some(v) => *v,
        };
        let mut cat = self.categories.get_mut(cat_idx).unwrap();
        let rule_idx = match self.rule_map.get(rule_name) {
            None => {
                let res = cat.len();
                self.rule_map.insert(rule_name, res);
                cat.push(Vec::new());
                res
            }
            Some(v) => *v,
        };
        cat[rule_idx].push(rule_value.convert());
        self
    }
}

pub struct RefFetcher<'a> {
    pub cat_map: &'a BTreeMap<String, usize>,
    pub rule_map: &'a BTreeMap<String, usize>,
}

impl<'a> RefFetcher<'a> {
    pub fn get_ref_info(&'a self, cat_name: &str, rule_name: &str) -> Option<RefInfo> {
        let cat_idx = self.cat_map.get(cat_name)?;
        let rule_key = format!("{}:{}", cat_name, rule_name);
        let rule_idx = self.rule_map.get(&rule_key)?;
        Some(RefInfo {
            cat_idx: *cat_idx,
            rule_idx: *rule_idx,
        })
    }
}

pub struct RefInfo {
    cat_idx: usize,
    rule_idx: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields;

    /*
    #[test]
    fn test_rule_set() {
        let rules = RuleSet::new()
            .set_category("test")
            .add_rule(rule!("rule", and!(sep = "", "hello", "there")));

        assert_eq!(rules.categories.len(), 1);
        assert_eq!(rules.categories[0].len(), 1);
    }
    */
}
