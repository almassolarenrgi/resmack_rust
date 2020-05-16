mod collections;
mod fields;
mod random;
mod rules;
mod types;

use crate::types::RuleBuilder;
use std::time;

macro_rules! _ref {
    ($ref_name:expr) => {
        reff!("test", $ref_name)
    };
}

pub fn main() {
    let rules = rules::RuleSet::new();
    let rules = rules
        .set_category(String::from("test"))
        .add_rule(rule!("Special", "SPECIAL ONE"))
        .add_rule(rule!("RefdRule", or!("Hello", "Blah", _ref!("Special"))))
        .add_rule(rule!("TestRule", _ref!("RefdRule"), "World"))
        .add_rule(rule!("TestRule2", _ref!("TestRule"), "World"))
        .add_rule(rule!("TestRule2", _ref!("TestRule"), "World"))
        .add_rule(rule!("TestRule2", or!(1, 2, 3, 4, 5)))
        .add_rule(rule!("TestRule2", 1000.5))
        .add_rule(rule!("TestRule2", "---World"));

    let start = time::Instant::now();
    let iters = 1_000_000;
    for _ in 0..iters {
        rules.build_rule(String::from("test"), String::from("TestRule2"));
    }
    let end = time::Instant::now();
    println!("{} iters/s", iters as f64 / (end - start).as_secs_f64());
}
