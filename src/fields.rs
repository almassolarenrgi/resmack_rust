#![macro_use]

use super::rules::{RefFetcher, RefInfo};

const SAFE_BUILD: bool = true;

/// Holds the final values that are used to build resulting data
pub enum Item<'a> {
    Direct(Vec<u8>),
    And(&'a And<'a>),
    Or(&'a Or<'a>),
    Ref(&'a Ref<'a>),
}

/// Used to convert the initial types used in the grammar from their source
/// types to one of the Item:: types.
pub trait Convertible {
    fn convert(&self) -> Item;
}

/*
/// Converts `&str` to an Item::Direct instance
impl Convertible for &str {
    fn convert(&self) -> Item {
        Item::Direct(self.as_bytes().to_vec())
    }
}
*/

/// Converts `&str` to an Item::Direct instance
impl Convertible for &str {
    fn convert(&self) -> Item {
        Item::Direct(self.as_bytes().to_vec())
    }
}

/// Converts `usize` (default for numbers) to an Item::Direct instance
impl Convertible for usize {
    fn convert(&self) -> Item {
        Item::Direct(self.to_string().as_bytes().to_vec())
    }
}

/// Converts `f64` (default for floats) to an Item::Direct instance
impl Convertible for f64 {
    fn convert(&self) -> Item {
        Item::Direct(self.to_string().as_bytes().to_vec())
    }
}

pub struct ItemBuilder<'a> {
    ref_fetcher: &'a RefFetcher<'a>,
}
impl<'a> ItemBuilder<'a> {
    pub fn build(&'a self, item: &'a Item<'a>, output: &mut Vec<u8>) {
        println!("Building an item");
        match item {
            Item::Direct(v) => {
                println!(
                    "Building direct item: {}",
                    std::str::from_utf8(&v[..]).unwrap()
                );
                if SAFE_BUILD {
                    Self::safe_build(v, output);
                } else {
                    Self::unsafe_build(v, output);
                }
            }
            Item::And(v) => v.build(self, output),
            Item::Or(v) => v.build(self, output),
            Item::Ref(v) => v.build(self, self.ref_fetcher, output),
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

pub struct And<'a> {
    sep: Item<'a>,
    items: Vec<Item<'a>>,
    use_sep: bool,
}

/// Converts `And` to an Item::And instance
impl<'a> Convertible for And<'a> {
    fn convert(&self) -> Item {
        Item::And(self)
    }
}

impl<'a> And<'a> {
    pub fn new(sep: &'a dyn Convertible) -> And<'a> {
        let use_sep = match sep.convert() {
            Item::Direct(v) => v.len() > 0,
            _ => true,
        };
        And {
            sep: sep.convert(),
            use_sep: use_sep,
            items: Vec::new(),
        }
    }

    pub fn add_item(mut self, item: &'a dyn Convertible) -> Self {
        self.items.push(item.convert());
        self
    }

    pub fn build(&'a self, builder: &'a ItemBuilder, output: &mut Vec<u8>) {
        let mut idx = 0;
        for item in self.items.iter() {
            if self.use_sep && idx > 0 {
                builder.build(&self.sep, output);
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
}

pub struct Or<'a> {
    pub choices: Vec<&'a Item<'a>>,
}

/// Converts `Or` to an Item::Or instance
impl<'a> Convertible for Or<'a> {
    fn convert(&self) -> Item {
        Item::Or(self)
    }
}

impl<'a> Or<'a> {
    pub fn build(&'a self, builder: &'a ItemBuilder, output: &mut Vec<u8>) {
        let choice_idx = 0;
        builder.build(
            self.choices.get(choice_idx).expect("Shouldn't fail"),
            output,
        );
    }
}

pub struct Ref<'a> {
    pub ref_cat: &'a str,
    pub ref_rule: &'a str,
    pub ref_info: Option<RefInfo>,
}

/// Converts `Ref` to an Item::Ref instance
impl<'a> Convertible for Ref<'a> {
    fn convert(&self) -> Item {
        Item::Ref(self)
    }
}

impl<'a> Ref<'a> {
    pub fn build(
        &'a self,
        builder: &'a ItemBuilder,
        ref_fetcher: &'a RefFetcher,
        output: &mut Vec<u8>,
    ) {
        unimplemented!()
        /*
        if let None = self.ref_info {
            self.ref_info = ref_fetcher.get_ref_info(&self.ref_cat, &self.ref_rule);
        }
        let rule_val = ref_fetcher.fetch_rule(self.ref_info.expect("Could not lookup rule"));
        builder.build(rule_val, output);
        */
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::str;

    macro_rules! build {
        ($item:expr) => {{
            let ref_fetcher: RefFetcher = RefFetcher {
                cat_map: &BTreeMap::new(),
                rule_map: &BTreeMap::new(),
            };
            let item_builder: ItemBuilder = ItemBuilder {
                ref_fetcher: &ref_fetcher,
            };
            let mut tmp_vec: Vec<u8> = Vec::new();
            $item.build(&item_builder, &mut tmp_vec);
            str::from_utf8(&tmp_vec[..]).unwrap().to_owned()
        }};
    }

    #[test]
    fn and_test() {
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
}
