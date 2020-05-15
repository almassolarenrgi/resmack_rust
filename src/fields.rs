#![macro_use]
use super::random;
use super::types::StringableVec;

pub struct And {
    pub items: StringableVec,
    pub sep: String,
}

impl And {
    pub fn new(sep: &str) -> And {
        And {
            items: vec![],
            sep: sep.to_string(),
        }
    }

    pub fn item(mut self, item: impl ToString + 'static) -> Self {
        self.items.push(Box::new(item));
        self
    }
}

impl ToString for And {
    fn to_string(&self) -> String {
        let mut res = String::new();
        for (idx, item) in self.items.iter().enumerate() {
            if idx > 0 {
                res += &self.sep;
            }
            res += &item.to_string();
        }
        res
    }
}

#[macro_export]
macro_rules! and {
    // all arguments specified, sep first
    ( sep=$sep:expr, $( $field:expr ),*) => {
        crate::fields::And::new($sep)
            $(.item($field))*
    };
    ( $( $field:expr ),*) => {
        and!(sep="", $($field), *)
    };
}

pub struct Or {
    pub items: StringableVec,
}

impl Or {
    pub fn new() -> Or {
        Or { items: vec![] }
    }

    pub fn item(mut self, item: impl ToString + 'static) -> Self {
        self.items.push(Box::new(item));
        self
    }
}

impl ToString for Or {
    fn to_string(&self) -> String {
        if self.items.len() == 0 {
            return String::from("");
        }
        let rand_idx = random::Rand::rand_int(0, self.items.len());
        let chosen_item = &self.items[rand_idx];
        chosen_item.to_string()
    }
}

#[macro_export]
macro_rules! or {
    // all arguments specified, sep first
    ( $( $field:expr ),*) => {
        crate::fields::Or::new()
            $(.item($field))*
    };
}

/*
pub struct Ref {
    refname: Box<dyn ToString>,
    rule_set:
}

impl Ref {
    pub fn new(refname: Box<ToString>) -> Ref {
        Ref { refname: refname }
    }
}

impl ToString for Ref {
    fn to_string() -> String {}
}
*/

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #[test]
    fn and_macro() {
        let and_test = and!("Hello", "World");
        assert_eq!(and_test.to_string(), "HelloWorld");
    }

    #[test]
    fn and_macro_with_sep() {
        let and_test = and!(sep = "*", "Hello", "World");
        assert_eq!(and_test.to_string(), "Hello*World");
    }

    #[test]
    fn or_macro() {
        let or_test = or!("Hello", "World");
        let built = or_test.to_string();

        let matches = or_test
            .items
            .iter()
            .filter(|item| item.to_string() == built)
            .count();
        assert_eq!(matches, 1);
    }
}
