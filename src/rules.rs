#![macro_use]
use std::collections::HashMap;

use super::collections::{new_fast_long_hash, new_fast_short_hash, FastLongHash, FastShortHash};
use super::random::Rand;
use super::types::*;

pub struct RuleSet {
    pub categories: FastShortHash<String, FastLongHash<String, Vec<Rule>>>,
    pub curr_category: String,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        RuleSet {
            categories: new_fast_short_hash(),
            curr_category: "".to_string(),
        }
    }

    pub fn set_category(mut self, cat: String) -> Self {
        self.curr_category = cat.clone();
        self
    }

    pub fn add_rule(mut self, rule: Rule) -> Self {
        let cat = &self.curr_category;
        if !self.categories.contains_key(cat) {
            self.categories.insert(cat.clone(), new_fast_long_hash());
        }
        let cat_map = self
            .categories
            .get_mut(cat)
            .expect("Could not lookup category");

        if !cat_map.contains_key(&rule.name) {
            cat_map.insert(rule.name.clone(), Vec::new());
        }
        let rule_list = cat_map
            .get_mut(&rule.name)
            .expect("Could not fetch rule list");

        rule_list.push(rule);

        self
    }

    pub fn get_rule<'a>(&'a self, cat: String, rule_name: String) -> Option<&'a Rule> {
        let category = match self.categories.get(&cat) {
            Some(v) => v,
            None => return None,
        };
        let rule_list = match category.get(&rule_name) {
            Some(v) => v,
            None => return Option::None,
        };
        let rand_idx = Rand::rand_int(0, rule_list.len());
        Some(&rule_list[rand_idx])
    }
}

impl RuleBuilder for RuleSet {
    fn build_rule<'a>(&'a self, cat: String, rule_name: String) -> String {
        let rule = self.get_rule(cat, rule_name).expect("Rule not found");
        rule.value.build(&Box::new(self))
    }
}

pub struct Rule {
    pub name: String,
    pub value: BoxedBuildable,
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
        fn build_rule<'a>(&'a self, cat: String, ref_name: String) -> String {
            panic!("Rule building not supported");
        }
    }
    impl FakeBuilder {
        fn new_boxed<'a>() -> BoxedRuleBuilder<'a> {
            let res: BoxedRuleBuilder = Box::new(&FakeBuilder {});
            res
        }
    }

    #[test]
    fn rule_set_creation() {
        let set = RuleSet::new();
        let set = set.add_rule(Rule {
            name: "Test Rule".to_string(),
            value: Box::new("Hello"),
        });
        if set.categories.contains_key("test") {
            println!("Hello");
        }
    }

    #[test]
    fn rule_macros() {
        let rules = RuleSet::new();
        let rules = rules
            .set_category("test".to_string())
            .add_rule(rule!("TestRule", "Hello", "World"))
            .add_rule(rule!("TestRule", "Hello", "world"));
        assert_eq!(rules.categories.contains_key("test"), true);

        let test_map = rules.categories.get("test").unwrap();
        assert_eq!(test_map.contains_key("TestRule"), true);

        let test_rules = test_map.get("TestRule").unwrap();
        assert_eq!(test_rules.len(), 2);
    }

    #[test]
    fn rule_set_chooses_random_rule() {
        let mut counts: HashMap<String, u32> = HashMap::new();
        let faker = FakeBuilder::new_boxed();

        let rules = RuleSet::new();
        let rules = rules
            .set_category("test".to_string())
            .add_rule(rule!("TestRule", "Hello", "World"))
            .add_rule(rule!("TestRule", "Goodbye", "Hello"));

        for _ in 0..10_000 {
            let res = rules
                .get_rule(String::from("test"), String::from("TestRule"))
                .unwrap()
                .value
                .build(&faker);

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
        let rules = RuleSet::new();
        let rules = rules
            .set_category(String::from("test"))
            .add_rule(rule!("RefdRule", "Hello"))
            .add_rule(rule!(
                "TestRule",
                fields::Ref::new(String::from("test"), String::from("RefdRule")),
                "World"
            ));
        let res = rules.build_rule(String::from("test"), String::from("TestRule"));
        assert_eq!(res, "HelloWorld");
    }

    #[test]
    fn rule_ref_macro_test() {
        let rules = RuleSet::new();
        let rules = rules
            .set_category(String::from("test"))
            .add_rule(rule!("RefdRule", "Hello"))
            .add_rule(rule!("TestRule", reff!("test", "RefdRule"), "World"))
            .add_rule(rule!("TestRule2", reff!("test", "TestRule"), "World"));

        let res = rules.build_rule(String::from("test"), String::from("TestRule"));
        assert_eq!(res, "HelloWorld");

        let res = rules.build_rule(String::from("test"), String::from("TestRule2"));
        assert_eq!(res, "HelloWorldWorld");
    }
}
