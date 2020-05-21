#![macro_use]

use std::collections::BTreeMap;

use crate::fields::{Convertible, Item, ItemBuilder};

pub struct RuleSet {
    cat_map: BTreeMap<String, usize>,
    rule_map: BTreeMap<String, usize>,
    pub categories: Vec<Vec<Vec<Item>>>,
    curr_cat: String,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        RuleSet {
            cat_map: BTreeMap::new(),
            rule_map: BTreeMap::new(),
            categories: Vec::new(),
            curr_cat: "".to_string(),
        }
    }

    pub fn set_category<T>(&mut self, cat: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.curr_cat = cat.into();
        self
    }

    pub fn add_rule<T, S>(&mut self, rule_name: S, rule_value: T) -> &mut Self
    where
        S: Into<String>,
        T: Convertible,
    {
        let rule_name = rule_name.into();

        let cat_idx = match self.cat_map.get(&self.curr_cat) {
            None => {
                let res = self.categories.len();
                self.cat_map.insert(self.curr_cat.clone(), res);
                self.categories.push(Vec::new());
                res
            }
            Some(v) => *v,
        };
        let mut cat = self.categories.get_mut(cat_idx).unwrap();
        let rule_idx = match self.rule_map.get(&rule_name) {
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

    pub fn finalize(&mut self) {
        let ref_fetcher = RefFetcher {
            cat_map: &self.cat_map,
            rule_map: &self.rule_map,
            categories: &self.categories,
        };

        for cat in self.categories.iter_mut() {
            for rule_list in cat.iter_mut() {
                for rule_opt in rule_list.iter_mut() {}
            }
        }
    }

    pub fn get_rule<'a, T>(&'a self, cat_name: T, rule_name: T) -> Option<&'a Item>
    where
        T: Into<String>,
    {
        let cat_idx = *self.cat_map.get(&cat_name.into())?;
        let rule_idx = *self.rule_map.get(&rule_name.into())?;
        // TODO random idx here
        let rand_idx = 0;
        Some(self.categories.get(cat_idx)?.get(rule_idx)?.get(rand_idx)?)
    }

    pub fn build_rule<'a, T>(&'a self, cat_name: T, rule_name: T, output: &mut Vec<u8>)
    where
        T: Into<String>,
    {
        let builder = self.new_builder();
        let rule = self
            .get_rule(cat_name, rule_name)
            .expect("Rule does not exist!");
        builder.build(rule, output);
    }

    fn new_builder(&self) -> ItemBuilder {
        ItemBuilder {
            ref_fetcher: RefFetcher {
                categories: &self.categories,
                cat_map: &self.cat_map,
                rule_map: &self.rule_map,
            },
        }
    }
}

pub struct RefFetcher<'a> {
    pub cat_map: &'a BTreeMap<String, usize>,
    pub rule_map: &'a BTreeMap<String, usize>,
    pub categories: &'a Vec<Vec<Vec<Item>>>,
}

impl<'a> RefFetcher<'a> {
    pub fn get_ref_info<T>(&'a self, cat_name: T, rule_name: T) -> Option<RefInfo>
    where
        T: Into<String>,
    {
        let cat_name = cat_name.into();
        let rule_name = rule_name.into();

        let cat_idx = *self.cat_map.get(&cat_name)?;
        let rule_key = format!("{}:{}", cat_name, rule_name);
        let rule_idx = *self.rule_map.get(&rule_key)?;
        Some(RefInfo {
            cat_idx: cat_idx,
            rule_idx: rule_idx,
        })
    }

    pub fn fetch_rule(&'a self, cat_idx: usize, rule_idx: usize) -> Option<&Item> {
        let cat = self.categories.get(cat_idx)?;
        let rules = cat.get(rule_idx)?;
        let res = rules.get(0)?;
        Some(res)
    }
}

pub struct RefInfo {
    pub cat_idx: usize,
    pub rule_idx: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields;
    use std::str;

    #[test]
    fn test_rule_set() {
        let mut rules = RuleSet::new();
        let rules = rules
            .set_category("test")
            .add_rule("rule", and!(sep = "", "hello", "there"));
        rules.finalize();

        assert_eq!(rules.categories.len(), 1);
        assert_eq!(rules.categories[0].len(), 1);

        let rule = rules.get_rule("test", "rule");
        assert_eq!(rule.is_some(), true);
    }

    #[test]
    fn test_rule_build() {
        let mut rules = RuleSet::new();
        let rules = rules
            .set_category("test")
            .add_rule("rule", and!("hello", "there"))
            .add_rule("rule2", and!("oogah", "boogah"));
        let mut output: Vec<u8> = Vec::new();
        rules.build_rule("test", "rule2", &mut output);
        assert_eq!(str::from_utf8(&output[..]).unwrap(), "oogahboogah");
    }
}
