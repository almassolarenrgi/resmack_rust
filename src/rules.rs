#![macro_use]
use std::collections::HashMap;

use super::random::Rand;
use super::types::StringableVec;

pub struct RuleSet {
    pub categories: HashMap<String, HashMap<String, Vec<Rule>>>,
    pub curr_category: String,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        RuleSet {
            categories: HashMap::new(),
            curr_category: "".to_string(),
        }
    }

    pub fn set_category(mut self, cat: String) -> Self {
        self.curr_category = cat;
        self
    }

    pub fn add_rule(mut self, rule: Rule) -> Self {
        let cat = &self.curr_category;
        if !self.categories.contains_key(cat) {
            self.categories.insert(cat.clone(), HashMap::new());
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

    pub fn get_rule<'a>(&'a mut self, cat: String, rule_name: String) -> Option<&'a Rule> {
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

pub struct Rule {
    pub name: String,
    pub value: Box<dyn ToString>,
}

#[macro_export]
macro_rules! rule {
    ( $name:expr, $( $item:expr ), *) => {
        Rule {
            name: $name.to_string(),
            value: Box::new(and!( $($item), *)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let rules = RuleSet::new();
        let mut rules = rules
            .set_category("test".to_string())
            .add_rule(rule!("TestRule", "Hello", "World"))
            .add_rule(rule!("TestRule", "Goodbye", "Hello"));

        for _ in 0..10_000 {
            let res = rules
                .get_rule(String::from("test"), String::from("TestRule"))
                .unwrap()
                .value
                .to_string();

            *counts.entry(res).or_insert(0) += 1;
        }

        assert_eq!(counts.len(), 2);

        let v1 = *counts.get("HelloWorld").unwrap() as f32;
        let v2 = *counts.get("GoodbyeHello").unwrap() as f32;
        let diff: f32 = (1.0 - (v1 / v2)).abs();
        assert_eq!(diff < 0.1, true); // should be roughly the same probabilities
    }
}
