#![macro_use]
use super::random;
use super::rules;
use super::rules::RuleSet;
use super::types::*;

pub struct And {
    pub items: BuildableVec,
    pub sep: String,
}

// ----------------------------------------------------------------------------

impl And {
    pub fn new(sep: &str) -> And {
        And {
            items: vec![],
            sep: sep.to_string(),
        }
    }

    pub fn item(mut self, item: impl Buildable + 'static) -> Self {
        self.items.push(Box::new(item));
        self
    }
}

impl Buildable for And {
    fn build(&self, output: &mut String, cb: &dyn RuleBuilder) {
        let mut res = String::new();
        for (idx, item) in self.items.iter().enumerate() {
            if idx > 0 {
                output.push_str(&self.sep);
            }
            item.build(output, cb);
        }
    }

    fn finalize(&mut self, info_fetcher: &dyn InfoFetcher) {
        for item in self.items.iter_mut() {
            item.finalize(info_fetcher);
        }
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

// ----------------------------------------------------------------------------

pub struct Or {
    pub items: BuildableVec,
}

impl Or {
    pub fn new() -> Or {
        Or { items: vec![] }
    }

    pub fn item(mut self, item: impl Buildable + 'static) -> Self {
        self.items.push(Box::new(item));
        self
    }
}

impl Buildable for Or {
    fn build(&self, output: &mut String, cb: &dyn RuleBuilder) {
        if self.items.len() == 0 {
            return;
        }
        let rand_idx = random::Rand::rand_int(0, self.items.len());
        let chosen_item = &self.items[rand_idx];
        chosen_item.build(output, cb);
    }

    fn finalize(&mut self, info_fetcher: &dyn InfoFetcher) {
        for item in self.items.iter_mut() {
            item.finalize(info_fetcher);
        }
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

// ----------------------------------------------------------------------------

pub struct Ref {
    ref_name: String,
    ref_cat: String,
    ref_info: Option<RuleInfo>,
}

impl Ref {
    pub fn new(ref_cat: String, ref_name: String) -> Ref {
        Ref {
            ref_name: ref_name,
            ref_cat: ref_cat,
            ref_info: None,
        }
    }
}

impl Buildable for Ref {
    fn build(&self, output: &mut String, cb: &dyn RuleBuilder) {
        let ref_info = self
            .ref_info
            .expect("ref_info was None. Was the ruleset not finalized?");
        cb.get_rule(ref_info)
            .expect("Rule does not exist")
            .value
            .build(output, cb);
    }

    fn finalize(&mut self, info_fetcher: &dyn InfoFetcher) {
        match self.ref_info {
            Some(_) => return,
            None => {
                self.ref_info =
                    info_fetcher.get_rule_info(self.ref_cat.clone(), self.ref_name.clone());
            }
        }
    }
}

#[macro_export]
macro_rules! reff {
    // all arguments specified, sep first
    ( $cat:expr, $ref_name:expr ) => {
        crate::fields::Ref::new($cat.to_string(), $ref_name.to_string())
    };
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules;

    struct FakeBuilder {}
    impl RuleBuilder for FakeBuilder {
        fn build_rule_slow<'a>(&'a mut self, _: String, _: String) -> String {
            panic!("Rule building not supported");
        }
        fn get_rule<'a>(&'a self, rule_info: RuleInfo) -> Option<&rules::Rule> {
            panic!("Rule building not supported");
        }
        fn build_rule<'a>(&'a self, _: RuleInfo) -> String {
            panic!("Rule building not supported");
        }
    }
    impl FakeBuilder {
        fn as_rule_builder(&self) -> &dyn RuleBuilder {
            self
        }
    }

    #[test]
    fn and_macro() {
        let faker = FakeBuilder {};
        let and_test = and!("Hello", "World");
        assert_eq!(and_test.build_direct(faker.as_rule_builder()), "HelloWorld");
    }

    #[test]
    fn and_macro_with_sep() {
        let faker = FakeBuilder {};
        let and_test = and!(sep = "*", "Hello", "World");
        assert_eq!(
            and_test.build_direct(faker.as_rule_builder()),
            "Hello*World"
        );
    }

    #[test]
    fn or_macro() {
        let faker = FakeBuilder {};
        let or_test = or!("Hello", "World");
        let built = or_test.build_direct(faker.as_rule_builder());

        let matches = or_test
            .items
            .iter()
            .filter(|item| item.build_direct(faker.as_rule_builder()) == built)
            .count();
        assert_eq!(matches, 1);
    }
}
