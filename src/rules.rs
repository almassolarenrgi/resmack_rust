#![macro_use]
use std::collections::HashMap;

use super::fields;
use super::types::StringableVec;

type Callback = fn();

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

    pub fn with_category(&mut self, cat: &str, c: Callback) {
        let tmp_cat = self.curr_category.clone();
        self.curr_category = cat.to_string();
        (c)();
        self.curr_category = tmp_cat;
    }

    pub fn add_rule(&mut self, cat: String, rule: Rule) -> &Self {
        if !self.categories.contains_key(&cat) {
            self.categories.insert(cat.clone(), HashMap::new());
        }
        let cat_map = self
            .categories
            .get_mut(&cat)
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
        let mut set = RuleSet::new();
        set.add_rule(
            "test".to_string(),
            Rule {
                name: "Test Rule".to_string(),
                value: Box::new("Hello"),
            },
        );

        if set.categories.contains_key("test") {
            println!("Hello");
        }
    }

    #[test]
    fn rule_macros() {
        let mut rules = RuleSet::new();
        rules.with_category("test", || {
            rule!("TestRule", "Hello", "World");
            rule!("TestRule", "Hello", or!("Food", "Beer"));
            rule!("TestRule", and!("test"));
        });
    }
}
