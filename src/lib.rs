mod fields;
mod random;

pub struct Rule<'a> {
    pub name: &'a str,
    pub value: Vec<Box<dyn ToString>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let and_test = fields::And {
            items: vec![Box::new(String::from("Hello"))],
            sep: String::from("|"),
        };
        println!("{}", and_test.to_string());
    }
}
