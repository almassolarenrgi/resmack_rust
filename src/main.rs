mod fields;
mod rules;

use crate::fields::*;
use crate::rules::*;

use std::time;

macro_rules! _ref {
    ($ref_name:expr) => {
        reff!("test", $ref_name)
    };
}

pub fn main() {
    let mut rules = RuleSet::new();
    let rules = rules
        .set_category("test")
        .add_rule("Special", "SPECIAL ONE")
        .add_rule("RefdRule", or!("Hello", "Blah", _ref!("Special")))
        .add_rule("TestRule", and!(_ref!("RefdRule"), "World"))
        .add_rule("TestRule2", and!(_ref!("TestRule"), "World"))
        .add_rule("TestRule2", and!(_ref!("TestRule"), "World"))
        .add_rule("TestRule2", and!(or!(1, 2, 3, 4, 5)))
        .add_rule("TestRule2", and!(1000.5))
        .add_rule("TestRule2", "---World");
    rules.finalize();

    let start = time::Instant::now();
    let mut total_size: usize = 0;
    let total_seconds = 3;
    let ref_info = rules
        .get_ref_info("test", "TestRule2")
        .expect("Should exist");
    let mut iters: usize = 0;
    let mut output: Vec<u8> = Vec::new();
    while (time::Instant::now() - start).as_millis() < (total_seconds * 1000) {
        rules.build_rule(&ref_info, &mut output);
        iters += 1;
    }
    total_size += output.len();
    let end = time::Instant::now();
    println!(
        "{} MiB/s, {} iters/s",
        (total_size as f64 / (1024.0 * 1024.0)) / (end - start).as_secs_f64(),
        (iters as f64 / (end - start).as_secs_f64()),
    );
}
