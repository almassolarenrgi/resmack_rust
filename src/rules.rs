#![macro_use]
use std::collections::HashMap;

use super::collections::{new_fast_long_hash, new_fast_short_hash, FastLongHash, FastShortHash};
use super::random::Rand;
use super::types::*;

pub struct RuleSet {
    pub cat_names: FastShortHash<String, usize>,
    // key is "category:rulename"
    pub rule_names: FastShortHash<String, usize>,

    /*
     *  // categories
     *  [
     *      // category, idx = 0
     *      [
     *          // rule, idx = 0
     *          [ Rule1, Rule2, Rule3 ],
     *      ]
     *  ]
     */
    pub categories: Vec<Vec<Vec<Rule>>>,
    pub curr_category: String,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        RuleSet {
            cat_names: new_fast_short_hash(),
            rule_names: new_fast_short_hash(),

            categories: Vec::new(),
            curr_category: "".to_string(),
        }
    }

    pub fn finalize(&mut self) {
        let info_fetcher: RuleInfoFetcher = RuleInfoFetcher::new(&self.cat_names, &self.rule_names);

        // resolve all Refs that have been added to the ruleset
        for cat in self.categories.iter_mut() {
            for rule_list in cat.iter_mut() {
                for rule in rule_list.iter_mut() {
                    rule.finalize(&info_fetcher);
                }
            }
        }
    }

    fn as_rule_builder(&self) -> &dyn RuleBuilder {
        self
    }

    fn as_rule_builder_mut(&mut self) -> &dyn RuleBuilder {
        self
    }

    pub fn set_category(mut self, cat: String) -> Self {
        self.curr_category = cat.clone();
        self
    }

    pub fn add_rule(mut self, rule: Rule) -> Self {
        let cat = &self.curr_category;
        let cat_idx = match self.cat_names.get(cat) {
            Some(v) => *v,
            None => {
                let tmp_idx = self.categories.len();
                self.cat_names.insert(cat.clone(), tmp_idx);
                self.categories.push(Vec::new());
                tmp_idx
            }
        };

        let cat_vec = self.categories.get_mut(cat_idx).expect("Shouldn't happen");

        let rule_key = format!("{}:{}", cat, rule.name);
        let rule_idx = match self.rule_names.get(&rule_key) {
            Some(v) => *v,
            None => {
                let tmp_idx = cat_vec.len();
                self.rule_names.insert(rule_key, tmp_idx);
                cat_vec.push(Vec::new());
                tmp_idx
            }
        };

        let rule_list = cat_vec.get_mut(rule_idx).expect("Shouldn't happen");
        rule_list.push(rule);

        self
    }

    pub fn get_rule_info(&self, cat_name: String, rule_name: String) -> Option<RuleInfo> {
        let info_fetcher: RuleInfoFetcher = RuleInfoFetcher::new(&self.cat_names, &self.rule_names);
        info_fetcher.get_rule_info(cat_name, rule_name)
    }

    pub fn get_rule<'a>(&'a self, info: RuleInfo) -> Option<&'a Rule> {
        let cat_list = self.categories.get(info.cat_idx)?;
        let rule_list = cat_list.get(info.rule_idx)?;
        let rand_idx = Rand::rand_int(0, rule_list.len());
        Some(&rule_list[rand_idx])
    }
}

impl RuleBuilder for RuleSet {
    fn build_rule<'a>(&self, rule_info: RuleInfo) -> String {
        let rule = self.get_rule(rule_info).expect("Rule not found");
        rule.value.build(self.as_rule_builder())
    }

    fn build_rule_slow<'a>(&mut self, cat: String, rule_name: String) -> String {
        let info_fetcher: RuleInfoFetcher = RuleInfoFetcher::new(&self.cat_names, &self.rule_names);
        self.build_rule(
            info_fetcher
                .get_rule_info(cat, rule_name)
                .expect("Rule not found"),
        )
    }
}

pub struct RuleInfoFetcher<'a> {
    cat_names: &'a FastShortHash<String, usize>,
    rule_names: &'a FastShortHash<String, usize>,
}

impl<'a> RuleInfoFetcher<'a> {
    fn new<'b>(
        cat_names: &'b FastShortHash<String, usize>,
        rule_names: &'b FastShortHash<String, usize>,
    ) -> RuleInfoFetcher<'b> {
        RuleInfoFetcher {
            cat_names: cat_names,
            rule_names: rule_names,
        }
    }
}

impl<'a> InfoFetcher for RuleInfoFetcher<'a> {
    fn get_rule_info(&self, cat: String, rule_name: String) -> Option<RuleInfo> {
        let cat_idx: usize = *self.cat_names.get(&cat)?;
        let rule_idx = *self.rule_names.get(&format!("{}:{}", cat, rule_name))?;
        Some(RuleInfo { cat_idx, rule_idx })
    }
}

pub struct Rule {
    pub name: String,
    pub value: BoxedBuildable,
}

impl Rule {
    pub fn finalize(&mut self, info_fetcher: &dyn InfoFetcher) {
        self.value.finalize(info_fetcher);
    }
}

#[macro_export]
macro_rules! rule {
    ( $name:expr, $( $item:expr ), *) => {
        crate::rules::Rule {
            name: $name.to_string(),
            value: Box::new(and!( $($item), *)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields;
    use crate::types;
    use crate::types::*;

    struct FakeBuilder {}
    impl RuleBuilder for FakeBuilder {
        fn build_rule_slow<'a>(&'a mut self, cat: String, rule_name: String) -> String {
            panic!("Rule building not supported");
        }
        fn build_rule<'a>(&'a self, rule_info: RuleInfo) -> String {
            panic!("Rule building not supported");
        }
    }
    impl FakeBuilder {
        fn as_rule_builder(&self) -> &dyn RuleBuilder {
            self
        }
    }

    #[test]
    fn rule_set_creation() {
        let mut set = RuleSet::new();
        set = set.set_category(String::from("test"));
        set = set.add_rule(Rule {
            name: "Test Rule".to_string(),
            value: Box::new("Hello"),
        });
        assert_eq!(set.cat_names.contains_key("test"), true);
    }

    #[test]
    fn rule_macros() {
        let rules = RuleSet::new();
        let rules = rules
            .set_category("test".to_string())
            .add_rule(rule!("TestRule", "Hello", "World"))
            .add_rule(rule!("TestRule", "Hello", "world"));
        assert_eq!(rules.cat_names.contains_key("test"), true);

        let rule_key = "test:TestRule";
        assert_eq!(rules.rule_names.contains_key(rule_key), true);

        let rule_info = rules
            .get_rule_info(String::from("test"), String::from("TestRule"))
            .expect("Should exist");
        let cat_list = rules
            .categories
            .get(rule_info.cat_idx)
            .expect("Should exist");
        let rule_list = cat_list.get(rule_info.rule_idx).expect("Should exist");
        assert_eq!(rule_list.len(), 2);
    }

    #[test]
    fn rule_set_chooses_random_rule() {
        let mut counts: HashMap<String, u32> = HashMap::new();
        let faker = FakeBuilder {};

        let rules = RuleSet::new();
        let rules = rules
            .set_category("test".to_string())
            .add_rule(rule!("TestRule", "Hello", "World"))
            .add_rule(rule!("TestRule", "Goodbye", "Hello"));

        let rule_info = rules
            .get_rule_info(String::from("test"), String::from("TestRule"))
            .expect("Should exist");

        for _ in 0..10_000 {
            let res = rules.build_rule(rule_info);
            *counts.entry(res).or_insert(0) += 1;
        }

        assert_eq!(counts.len(), 2);

        let v1 = *counts.get("HelloWorld").unwrap() as f32;
        let v2 = *counts.get("GoodbyeHello").unwrap() as f32;
        let diff: f32 = (1.0 - (v1 / v2)).abs();
        assert_eq!(diff < 0.1, true); // should be roughly the same probabilities
    }

    #[test]
    fn rule_set_ref() {
        let mut rules = RuleSet::new();
        rules = rules
            .set_category(String::from("test"))
            .add_rule(rule!("RefdRule", "Hello"))
            .add_rule(rule!(
                "TestRule",
                fields::Ref::new(String::from("test"), String::from("RefdRule")),
                "World"
            ));
        rules.finalize();
        let rule_info = rules
            .get_rule_info(String::from("test"), String::from("TestRule"))
            .expect("Should exist");
        let res = rules.build_rule(rule_info);
        assert_eq!(res, "HelloWorld");
    }

    #[test]
    fn rule_ref_macro_test() {
        let rules = RuleSet::new();
        let mut rules = rules
            .set_category(String::from("test"))
            .add_rule(rule!("RefdRule", "Hello"))
            .add_rule(rule!("TestRule", reff!("test", "RefdRule"), "World"))
            .add_rule(rule!("TestRule2", reff!("test", "TestRule"), "World"));
        rules.finalize();

        let res = rules.build_rule_slow(String::from("test"), String::from("TestRule"));
        assert_eq!(res, "HelloWorld");

        let res = rules.build_rule_slow(String::from("test"), String::from("TestRule2"));
        assert_eq!(res, "HelloWorldWorld");
    }
}
