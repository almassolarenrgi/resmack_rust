mod fields;
mod random;

use std::time;

pub fn main() {
    let start = time::Instant::now();
    let iters = 0x100000;
    for i in (0..iters) {
        let and_test = fields::And {
            items: vec![
                Box::new(String::from("Hello")),
                Box::new(String::from("World")),
            ],
            sep: String::from("|"),
        };
        let or_test = fields::Or {
            items: vec![
                Box::new(String::from("Hello")),
                Box::new(String::from("World")),
                Box::new(and_test),
            ],
        };
        let res = or_test.to_string();
        println!("{}", res);
        break;
    }
    let end = time::Instant::now();
    println!("{} iters/s", iters as f64 / (end - start).as_secs_f64());
}
