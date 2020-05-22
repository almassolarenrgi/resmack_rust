#![macro_use]

use std::collections::BTreeMap;

use super::fields::{Convertible, Item, ItemBuilder};
use super::random::Rand;

pub struct RuleSet {
    pub cat_map: BTreeMap<String, usize>,
    pub rule_map: BTreeMap<String, usize>,
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
        let cat = self.categories.get_mut(cat_idx).unwrap();

        let rule_key = format!("{}:{}", self.curr_cat, rule_name);
        let rule_idx = match self.rule_map.get(&rule_key) {
            None => {
                let res = cat.len();
                self.rule_map.insert(rule_key, res);
                cat.push(Vec::new());
                res
            }
            Some(v) => *v,
        };
        cat[rule_idx].push(rule_value.convert());
        self
    }

    pub fn finalize(&mut self) {
        let fetcher = RefFetcher {
            cat_map: &self.cat_map,
            rule_map: &self.rule_map,
        };

        for cat in self.categories.iter_mut() {
            for rule_list in cat.iter_mut() {
                for rule_opt in rule_list.iter_mut() {
                    fetcher.finalize(rule_opt);
                }
            }
        }
    }

    pub fn get_ref_info<T>(&self, cat_name: T, rule_name: T) -> Option<RefInfo>
    where
        T: Into<String>,
    {
        let cat_name = cat_name.into();
        let rule_name = rule_name.into();

        let cat_idx = *self.cat_map.get(&cat_name)?;
        let rule_idx = *self.rule_map.get(&format!("{}:{}", cat_name, rule_name))?;

        Some(RefInfo { cat_idx, rule_idx })
    }

    pub fn build_rule(&self, ref_info: &RefInfo, output: &mut Vec<u8>, rand: &mut Rand) {
        let builder = ItemBuilder {
            categories: &self.categories,
        };

        let rule_list = self
            .categories
            .get(ref_info.cat_idx)
            .expect("Invalid RefInfo")
            .get(ref_info.rule_idx)
            .expect("Invalid RefInfo");
        let rand_idx = if rule_list.len() == 1 {
            0
        } else {
            rand.rand_int(0, rule_list.len())
        };
        let rule = rule_list.get(rand_idx).expect("Random index was incorrect");
        builder.build(rule, output, rand);
    }

    #[allow(dead_code)]
    pub fn get_rule_slow<'a, T>(
        &'a self,
        cat_name: T,
        rule_name: T,
        rand: &mut Rand,
    ) -> Option<&'a Item>
    where
        T: Into<String>,
    {
        let cat_name = cat_name.into();
        let rule_name = rule_name.into();

        let ref_info = self.get_ref_info(cat_name, rule_name)?;

        let rule_list = self
            .categories
            .get(ref_info.cat_idx)?
            .get(ref_info.rule_idx)?;

        let rand_idx = if rule_list.len() == 1 {
            0
        } else {
            rand.rand_int(0, rule_list.len())
        };

        Some(rule_list.get(rand_idx).expect("Random index was incorrect"))
    }

    #[allow(dead_code)]
    pub fn build_rule_slow<'a, T>(
        &'a self,
        cat_name: T,
        rule_name: T,
        output: &mut Vec<u8>,
        rand: &mut Rand,
    ) where
        T: Into<String>,
    {
        let builder = ItemBuilder {
            categories: &self.categories,
        };

        let rule = self
            .get_rule_slow(cat_name, rule_name, rand)
            .expect("Rule does not exist!");
        builder.build(rule, output, rand);
    }
}

pub struct RefFetcher<'a> {
    pub cat_map: &'a BTreeMap<String, usize>,
    pub rule_map: &'a BTreeMap<String, usize>,
}

impl<'a> RefFetcher<'a> {
    pub fn finalize(&self, item: &mut Item) {
        match item {
            Item::And(v) => v.finalize(&self),
            Item::Ref(v) => v.finalize(&self),
            Item::Or(v) => v.finalize(&self),
            _ => (),
        };
    }

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
}

pub struct RefInfo {
    pub cat_idx: usize,
    pub rule_idx: usize,
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
        let rules = rules
            .set_category("test")
            .add_rule("rule", and!(sep = "", "hello", "there"));
        rules.finalize();

        assert_eq!(rules.categories.len(), 1);
        assert_eq!(rules.categories[0].len(), 1);

        let rule = rules.get_rule_slow("test", "rule", &mut rand);
        assert_eq!(rule.is_some(), true);
    }

    #[test]
    fn test_rule_build() {
        let mut rules = RuleSet::new();
        let rules = rules
            .set_category("test")
            .add_rule("rule", and!("hello", "there"))
            .add_rule("rule2", and!("oogah", reff!("test", "rule"), "boogah"));
        let mut rand = Rand::new(0);
        rules.finalize();

        let mut output: Vec<u8> = Vec::new();
        rules.build_rule_slow("test", "rule2", &mut output, &mut rand);
        assert_eq!(
            str::from_utf8(&output[..]).unwrap(),
            "oogahhellothereboogah"
        );
    }
}
