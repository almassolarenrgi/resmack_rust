mod fields;
mod random;
mod rules;
mod types;

use std::time;

pub fn main() {
    let start = time::Instant::now();
    let iters = 1_000_000;
    for _ in 0..iters {
        let and_test = and!("Hello", "World");
        let or_test = or!("Hello", "World", and_test);
        or_test.to_string();
    }
    let end = time::Instant::now();
    println!("{} iters/s", iters as f64 / (end - start).as_secs_f64());
}
