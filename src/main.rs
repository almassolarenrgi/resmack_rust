mod fields;
mod random;
mod rules;

use crate::random::Rand;
use crate::rules::*;

use std::time;

pub fn main() {
    let mut rand = Rand::new(1337);
    let mut rules = RuleSet::new();
    let rules = rules
        .add_rule("PruneMe", reff!("unresolvable"))
        .add_rule("PruneMeToo", reff!("PruneMe"))
        .add_rule("Special", "SPECIAL ONE")
        .add_rule("RefdRule", or!("Hello", "Blah", reff!("Special")))
        .add_rule("TestRule", and!(reff!("RefdRule"), "World"))
        .add_rule("TestRule2", and!(reff!("TestRule"), "World"))
        .add_rule("TestRule2", and!(reff!("TestRule"), "World"))
        .add_rule("TestRule2", int!(min = 5, max = 1337))
        .add_rule(
            "TestRule2",
            and!(or!(
                1,
                2,
                3,
                4,
                5,
                string!(min = 5, max = 10, charset = "abcdefg")
            )),
        )
        .add_rule("TestRule2", and!(1000.5))
        .add_rule("TestRule2", "---World");
    rules.finalize();

    let ref_idx = rules.get_ref_idx("TestRule2").expect("Should exist");
    let mut iters: usize = 0;
    let mut output: Vec<u8> = Vec::new();
    let mut total_size: usize = 0;
    let start = time::Instant::now();

    //while (time::Instant::now() - start).as_millis() < (total_seconds * 1000) {
    loop {
        output.clear();
        rules.build_rule(ref_idx, &mut output, &mut rand);
        total_size += output.len();
        iters += 1;
        if iters % 0xfffff == 0 {
            let end = time::Instant::now();
            println!(
                "{} MiB/s, {} iters/s",
                (total_size as f64 / (1024.0 * 1024.0)) / (end - start).as_secs_f64(),
                (iters as f64 / (end - start).as_secs_f64()),
            );
        }
    }
}
