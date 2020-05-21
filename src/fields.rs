#![macro_use]

use super::rules::{RefFetcher, RefInfo};

const SAFE_BUILD: bool = true;

/// Holds the final values that are used to build resulting data
pub enum Item {
    Direct(Vec<u8>),
    And(And),
    Or(Or),
    Ref(Ref),
}

/// Used to convert the initial types used in the grammar from their source
/// types to one of the Item:: types.
pub trait Convertible: Sized {
    fn convert(self) -> Item;
}

/// Converts `String` to an Item::Direct instance
impl<'a> Convertible for String {
    fn convert(self) -> Item {
        Item::Direct(self.as_bytes().to_vec())
    }
}

/// Converts `String` to an Item::Direct instance
impl<'a> Convertible for &str {
    fn convert(self) -> Item {
        Item::Direct(self.as_bytes().to_vec())
    }
}

/// Converts `usize` (default for numbers) to an Item::Direct instance
impl<'a> Convertible for usize {
    fn convert(self) -> Item {
        Item::Direct(self.to_string().as_bytes().to_vec())
    }
}

/// Converts `usize` (default for numbers) to an Item::Direct instance
impl<'a> Convertible for i32 {
    fn convert(self) -> Item {
        Item::Direct(self.to_string().as_bytes().to_vec())
    }
}

/// Converts `f64` (default for floats) to an Item::Direct instance
impl<'a> Convertible for f64 {
    fn convert(self) -> Item {
        Item::Direct(self.to_string().as_bytes().to_vec())
    }
}

pub struct ItemBuilder<'a> {
    pub categories: &'a Vec<Vec<Vec<Item>>>,
}

impl<'a> ItemBuilder<'a> {
    pub fn build(&'a self, item: &'a Item, output: &mut Vec<u8>) {
        match item {
            Item::Direct(v) => self.direct_build(v, output),
            Item::And(v) => v.build(self, output),
            Item::Ref(v) => v.build(self, output),
            Item::Or(v) => v.build(self, output),
        }
    }

    pub fn fetch_rule(&'a self, cat_idx: usize, rule_idx: usize) -> Option<&Item> {
        let cat = self.categories.get(cat_idx)?;
        let rules = cat.get(rule_idx)?;
        let res = rules.get(0)?;
        Some(res)
    }

    pub fn direct_build(&'a self, v: &Vec<u8>, output: &mut Vec<u8>) {
        if SAFE_BUILD {
            Self::safe_build(v, output);
        } else {
            Self::unsafe_build(v, output);
        }
    }

    fn safe_build(item: &Vec<u8>, output: &mut Vec<u8>) {
        output.extend_from_slice(item);
    }
    fn unsafe_build(item: &Vec<u8>, output: &mut Vec<u8>) {
        unsafe {
            let old_size = output.len();
            let new_size = old_size + item.len();

            if new_size > output.capacity() {
                {
                    output.reserve(new_size - old_size);
                }
            }

            std::ptr::copy_nonoverlapping(
                item.as_ptr(),
                output.as_mut_ptr().offset(old_size as isize),
                item.len(),
            );
            output.set_len(new_size);
        }
    }
}

pub struct And {
    sep: Vec<u8>,
    items: Vec<Item>,
}

/// Converts `And` to an Item::And instance
impl Convertible for And {
    fn convert(self) -> Item {
        Item::And(self)
    }
}

impl And {
    pub fn new<T: Convertible>(sep: T) -> And {
        And {
            sep: match sep.convert() {
                Item::Direct(v) => v,
                _ => panic!("Separator may only be an Item::Direct"),
            },
            items: Vec::new(),
        }
    }

    pub fn add_item<T: Convertible>(mut self, item: T) -> Self {
        self.items.push(item.convert());
        self
    }

    pub fn finalize(&mut self, fetcher: &RefFetcher) {
        for item in self.items.iter_mut() {
            fetcher.finalize(item);
        }
    }

    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>) {
        let mut idx = 0;
        for item in self.items.iter() {
            if self.sep.len() > 0 && idx > 0 {
                builder.direct_build(&self.sep, output);
            }
            builder.build(item, output);
            idx += 1;
        }
    }
}

#[macro_export]
macro_rules! and {
    (sep = $sep:expr, $($item:expr),*) => {
        crate::fields::And::new($sep)
            $(.add_item($item))*
    };
    ($($item:expr),*) => {
        crate::fields::And::new("")
            $(.add_item($item))*
    };
}

pub struct Or {
    pub choices: Vec<Item>,
}

/// Converts `Or` to an Item::Or instance
impl Convertible for Or {
    fn convert(self) -> Item {
        Item::Or(self)
    }
}

impl Or {
    pub fn new() -> Or {
        Or {
            choices: Vec::new(),
        }
    }

    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>) {
        // TODO RANDOMNESS HERE
        let choice_idx = 0;
        builder.build(
            self.choices.get(choice_idx).expect("Shouldn't fail"),
            output,
        );
    }

    pub fn finalize(&mut self, fetcher: &RefFetcher) {
        for choice in self.choices.iter_mut() {
            fetcher.finalize(choice);
        }
    }

    pub fn add_item<T: Convertible>(mut self, choice: T) -> Self {
        self.choices.push(choice.convert());
        self
    }
}

#[macro_export]
macro_rules! or {
    ($($item:expr),*) => {
        crate::fields::Or::new()
            $(.add_item($item))*
    }
}

pub struct Ref {
    pub ref_cat: String,
    pub ref_rule: String,
    pub ref_info: Option<RefInfo>,
}

/// Converts `Ref` to an Item::Ref instance
impl Convertible for Ref {
    fn convert(self) -> Item {
        Item::Ref(self)
    }
}

impl Ref {
    pub fn new<T>(ref_cat: T, ref_rule: T) -> Ref
    where
        T: Into<String>,
    {
        Ref {
            ref_cat: ref_cat.into(),
            ref_rule: ref_rule.into(),
            ref_info: None,
        }
    }
    pub fn finalize(&mut self, ref_fetcher: &RefFetcher) {
        if let None = self.ref_info {
            self.ref_info = ref_fetcher.get_ref_info(&self.ref_cat, &self.ref_rule);
        }
    }
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>) {
        let (cat_idx, rule_idx) = match &self.ref_info {
            None => panic!("Rule was never resolved! Was finalize not called?"),
            Some(v) => (v.cat_idx, v.rule_idx),
        };
        let rule_val = builder
            .fetch_rule(cat_idx, rule_idx)
            .expect("Concrete reference no longer valid");
        builder.build(&rule_val, output);
    }
}

#[macro_export]
macro_rules! reff {
    ($cat:expr, $ref:expr) => {
        crate::fields::Ref::new($cat, $ref)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    macro_rules! build {
        ($item:expr) => {{
            let item_builder: ItemBuilder = ItemBuilder {
                categories: &Vec::new(),
            };
            let mut tmp_vec: Vec<u8> = Vec::new();
            $item.build(&item_builder, &mut tmp_vec);
            str::from_utf8(&tmp_vec[..]).unwrap().to_owned()
        }};
    }

    #[test]
    fn convert_string() {
        let item: Item = String::from("hello").convert();
        match item {
            Item::Direct(_) => (),
            _ => assert_eq!(false, true),
        };
    }

    #[test]
    fn convert_usize() {
        let item: Item = 12.convert();
        match item {
            Item::Direct(_) => (),
            _ => assert_eq!(false, true),
        };
    }

    #[test]
    fn convert_i32() {
        let item: Item = (-12).convert();
        match item {
            Item::Direct(_) => (),
            _ => assert_eq!(false, true),
        };
    }

    #[test]
    fn convert_f64() {
        let item: Item = 100.05.convert();
        match item {
            Item::Direct(_) => (),
            _ => assert_eq!(false, true),
        };
    }

    #[test]
    fn and_full() {
        let and = And::new("|").add_item("Test").add_item("yoyoy");
        let res = build!(and);
        assert_eq!(res, "Test|yoyoy");
    }

    #[test]
    fn and_macro() {
        let and = and!(sep = "", "hello", "there");
        let res = build!(and);
        assert_eq!(res, "hellothere");
    }

    #[test]
    fn and_test_sep() {
        let and = and!(sep = "|", "hello", "there");
        let res = build!(and);
        assert_eq!(res, "hello|there");
    }

    #[test]
    fn nested_and() {
        let and_inner = and!(sep = "|", "hello", "there");
        let and_outer = and!(sep = "-", "hello", and_inner, "there");
        let res = build!(and_outer);
        assert_eq!(res, "hello-hello|there-there");
    }

    #[test]
    fn test_or() {
        let or = or!("hello", "there");
        let res = build!(or);
        assert_eq!(res == "hello" || res == "there", true);
    }
}
