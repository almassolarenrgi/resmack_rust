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
        .add_rule("variable", "default_var")
        .add_rule("create_id", and!("var ", id!("variable"), " = 10;"))
        .add_rule("use_id", and!(reff!("variable"), " + 20;"))
        .add_rule("statement", or!(reff!("create_id"), reff!("use_id")));
    rules.finalize();

    let ref_idx = rules.get_ref_idx("statement").expect("Should exist");
    let mut iters: usize = 0;
    let mut output: Vec<u8> = Vec::new();
    let mut total_size: usize = 0;
    let start = time::Instant::now();

    //while (time::Instant::now() - start).as_millis() < (total_seconds * 1000) {
    loop {
        output.clear();
        rules.build_rule(ref_idx, &mut output, &mut rand, 3);
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
