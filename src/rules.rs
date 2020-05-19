use crate::fields::Item;
use std::collections::BTreeMap;

pub struct RuleSet<'a> {
    cat_map: BTreeMap<&'a str, usize>,
    rule_map: BTreeMap<&'a str, usize>,
    rules: Vec<Vec<Vec<&'a Item<'a>>>>,
}

impl<'a> RuleSet<'a> {
    fn add_rule(&'a mut self, cat_name: &'a str, rule_name: &'a str, value: &'a Item<'a>) {
        let cat_idx = match self.cat_map.get(cat_name) {
            None => {
                let res = self.rules.len();
                self.cat_map.insert(cat_name, res);
                self.rules.push(Vec::new());
                res
            }
            Some(v) => *v,
        };
        let mut cat = self.rules.get_mut(cat_idx).unwrap();
        let rule_idx = match self.rule_map.get(rule_name) {
            None => {
                let res = cat.len();
                self.rule_map.insert(rule_name, res);
                res
            }
            Some(v) => *v,
        };
        cat[rule_idx].push(value);
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
