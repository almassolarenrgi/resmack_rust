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
    let mut rules = rules::RuleSet::new();
    rules = rules
        .set_category(String::from("test"))
        .add_rule(rule!("Special", "SPECIAL ONE"))
        .add_rule(rule!("RefdRule", or!("Hello", "Blah", _ref!("Special"))))
        .add_rule(rule!("TestRule", _ref!("RefdRule"), "World"))
        .add_rule(rule!("TestRule2", _ref!("TestRule"), "World"))
        .add_rule(rule!("TestRule2", _ref!("TestRule"), "World"))
        .add_rule(rule!("TestRule2", or!(1, 2, 3, 4, 5)))
        .add_rule(rule!("TestRule2", 1000.5))
        .add_rule(rule!("TestRule2", "---World"));
    rules.finalize();

    let start = time::Instant::now();
    let mut total_size: usize = 0;
    let total_seconds = 3;
    let rule_info = rules
        .get_rule_info(String::from("test"), String::from("TestRule2"))
        .expect("Should exist");
    let mut iters: usize = 0;
    while (time::Instant::now() - start).as_millis() < (total_seconds * 1000) {
        let output = rules.build_rule(rule_info);
        total_size += output.len();
        iters += 1;
    }
    let end = time::Instant::now();
    println!(
        "{} MiB/s, {} iters/s",
        (total_size as f64 / (1024.0 * 1024.0)) / (end - start).as_secs_f64(),
        (iters as f64 / (end - start).as_secs_f64()),
    );
}
