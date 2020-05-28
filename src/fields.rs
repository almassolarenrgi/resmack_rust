#![macro_use]

use std::fmt;
use std::str;

use super::random::Rand;
use super::rules::{RefFetcher, RefInfo, RuleKeys};

const SAFE_BUILD: bool = true;

/// Holds the final values that are used to build resulting data
pub enum Item {
    Direct(Vec<u8>),
    And(And),
    Or(Or),
    Ref(Ref),
    Str(Str),
    Int(Int),
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Direct(v) => write!(f, "u8[{}]", str::from_utf8(v).unwrap()),
            Item::And(v) => v.fmt(f),
            Item::Or(v) => v.fmt(f),
            Item::Ref(v) => v.fmt(f),
            Item::Str(v) => v.fmt(f),
            Item::Int(v) => v.fmt(f),
        }
    }
}

/// Used to convert the initial types used in the grammar from their source
/// types to one of the Item:: types.
pub trait Convertible: Sized {
    fn convert(self) -> Item;
}

/// Converts `String` to an Item::Direct instance
impl<'a> Convertible for String {
    #[inline]
    fn convert(self) -> Item {
        Item::Direct(self.as_bytes().to_vec())
    }
}

/// Converts `String` to an Item::Direct instance
impl<'a> Convertible for &str {
    #[inline]
    fn convert(self) -> Item {
        Item::Direct(self.as_bytes().to_vec())
    }
}

/// Converts `usize` (default for numbers) to an Item::Direct instance
impl<'a> Convertible for usize {
    #[inline]
    fn convert(self) -> Item {
        Item::Direct(self.to_string().as_bytes().to_vec())
    }
}

/// Converts `usize` (default for numbers) to an Item::Direct instance
impl<'a> Convertible for i32 {
    #[inline]
    fn convert(self) -> Item {
        Item::Direct(self.to_string().as_bytes().to_vec())
    }
}

/// Converts `f64` (default for floats) to an Item::Direct instance
impl<'a> Convertible for f64 {
    #[inline]
    fn convert(self) -> Item {
        Item::Direct(self.to_string().as_bytes().to_vec())
    }
}

pub struct ItemBuilder<'a> {
    pub categories: &'a Vec<Vec<Vec<Item>>>,
}

impl<'a> ItemBuilder<'a> {
    #[inline]
    pub fn build(&'a self, item: &'a Item, output: &mut Vec<u8>, rand: &mut Rand) {
        match item {
            Item::Direct(v) => self.direct_build(v, output),
            Item::And(v) => v.build(self, output, rand),
            Item::Ref(v) => v.build(self, output, rand),
            Item::Or(v) => v.build(self, output, rand),
            Item::Str(v) => v.build(self, output, rand),
            Item::Int(v) => v.build(self, output, rand),
        }
    }

    #[inline]
    pub fn fetch_rule(
        &'a self,
        cat_idx: usize,
        mut rule_idx: usize,
        rand: &mut Rand,
    ) -> Option<&Item> {
        let cat = self.categories.get(cat_idx)?;
        if rule_idx == (RuleKeys::Any as usize) {
            rule_idx = rand.rand_u64(0, cat.len() as u64) as usize;
        }
        let rules = cat.get(rule_idx)?;
        let rand_idx = rand.rand_u64(0, rules.len() as u64) as usize;
        let res = rules.get(rand_idx)?;
        Some(res)
    }

    #[inline]
    pub fn direct_build(&'a self, v: &Vec<u8>, output: &mut Vec<u8>) {
        if SAFE_BUILD {
            Self::safe_build(v, output);
        } else {
            Self::unsafe_build(v, output);
        }
    }

    #[inline]
    fn safe_build(item: &Vec<u8>, output: &mut Vec<u8>) {
        output.extend(item);
    }

    #[inline]
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

// ----------------------------------------------------------------------------
// AND
// ----------------------------------------------------------------------------

pub struct And {
    sep: Vec<u8>,
    items: Vec<Item>,
}

/// Converts `And` to an Item::And instance
impl Convertible for And {
    #[inline]
    fn convert(self) -> Item {
        Item::And(self)
    }
}

impl fmt::Display for And {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "And<sep={} {}>",
            str::from_utf8(&self.sep).unwrap(),
            self.items
                .iter()
                .map({ |x| format!("{}", x) })
                .collect::<Vec<String>>()
                .join(", ")
        )
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

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) {
        let mut idx = 0;
        for item in self.items.iter() {
            if self.sep.len() > 0 && idx > 0 {
                builder.direct_build(&self.sep, output);
            }
            builder.build(item, output, rand);
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

// ----------------------------------------------------------------------------
// OR
// ----------------------------------------------------------------------------

pub struct Or {
    pub choices: Vec<Item>,
}

/// Converts `Or` to an Item::Or instance
impl Convertible for Or {
    fn convert(self) -> Item {
        Item::Or(self)
    }
}

impl fmt::Display for Or {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Or<{}>",
            self.choices
                .iter()
                .map({ |x| format!("{}", x) })
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Or {
    pub fn new() -> Or {
        Or {
            choices: Vec::new(),
        }
    }

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) {
        let choice_idx = rand.rand_u64(0, self.choices.len() as u64);
        builder.build(
            self.choices
                .get(choice_idx as usize)
                .expect("Shouldn't fail"),
            output,
            rand,
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

// ----------------------------------------------------------------------------
// Ref
// ----------------------------------------------------------------------------

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

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ref<{}:{}>", self.ref_cat, self.ref_rule)
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

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) {
        let (cat_idx, rule_idx) = match &self.ref_info {
            None => panic!(format!(
                "{} was never resolved! Was finalize not called?",
                self
            )),
            Some(v) => (v.cat_idx, v.rule_idx),
        };
        let rule_val = builder
            .fetch_rule(cat_idx, rule_idx, rand)
            .expect("Concrete reference no longer valid");
        builder.build(&rule_val, output, rand);
    }
}

#[macro_export]
macro_rules! reff {
    ($cat:expr, $ref:expr) => {
        crate::fields::Ref::new($cat, $ref)
    };
}

// ----------------------------------------------------------------------------
// Str
// ----------------------------------------------------------------------------

/// The Str struct will be able to create a random string in the range
/// [min, max] using the specified charset
pub struct Str {
    min: u64,
    max: u64,
    charset: Vec<u8>,
}

impl Convertible for Str {
    fn convert(self) -> Item {
        Item::Str(self)
    }
}

impl fmt::Display for Str {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Str<min={} max={} charset={:?}>",
            self.min,
            self.max,
            str::from_utf8(&self.charset).unwrap(),
        )
    }
}

impl Str {
    pub fn new<T>(min: u64, max: u64, charset: T) -> Self
    where
        T: Into<Vec<u8>>,
    {
        Str {
            min,
            max,
            charset: charset.into(),
        }
    }

    // no finalize needed

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) {
        let len = rand.rand_u64(self.min, self.max) as usize;
        let mut res: Vec<u8> = vec![0; len];
        for idx in 0..len {
            let rand_idx = rand.rand_u64(0, self.charset.len() as u64) as usize;
            res[idx] = self.charset[rand_idx];
        }
        builder.direct_build(&res, output);
    }
}

#[macro_export]
macro_rules! string {
    (min = $min:expr, max = $max:expr, charset = $charset:expr) => {
        crate::fields::Str::new($min, $max, $charset)
    };
    (max = $max: expr, charset = $charset:expr) => {
        string!(min = 0, max = $max, charset = $charset)
    };
    ($charset:expr) => {
        string!(max = 20, charset = $charset)
    };
    () => {
        string!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
    };
}

// ----------------------------------------------------------------------------
// Int
// ----------------------------------------------------------------------------

/// The Int struct will be able to create a random i64 in the range
/// [min, max]
pub struct Int {
    min: i64,
    max: i64,
}

impl Convertible for Int {
    fn convert(self) -> Item {
        Item::Int(self)
    }
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Int<min={} max={}>", self.min, self.max,)
    }
}

impl Int {
    pub fn new(min: i64, max: i64) -> Self {
        Int { min, max }
    }

    // no finalize needed

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) {
        let val = rand.rand_i64(self.min, self.max);
        builder.direct_build(&val.to_string().as_bytes().to_vec(), output);
    }
}

#[macro_export]
macro_rules! int {
    (min = $min:expr, max = $max:expr) => {
        crate::fields::Int::new($min, $max)
    };
    (max = $max: expr) => {
        int!(min = 0, max = $max)
    };
    () => {
        int!(max = 1000)
    };
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Rand;
    use std::str;
    use std::time::{SystemTime, UNIX_EPOCH};

    macro_rules! build {
        ($item:expr) => {{
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let item_builder: ItemBuilder = ItemBuilder {
                categories: &Vec::new(),
            };
            let mut rand = Rand::new(since_the_epoch.as_nanos());
            let mut tmp_vec: Vec<u8> = Vec::new();
            $item.build(&item_builder, &mut tmp_vec, &mut rand);
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

    #[test]
    fn test_str_full_macro() {
        let charset = "hello";
        let val = string!(min = 1, max = 5, charset = charset);
        for _ in 0..100 {
            let res = build!(val);
            assert_eq!(res.chars().all(|x| charset.contains(x)), true);
        }
    }

    #[test]
    fn test_str_max_charset_macro() {
        let charset = "hello";
        let val = string!(max = 5, charset = charset);
        for _ in 0..100 {
            let res = build!(val);
            assert_eq!(res.chars().all(|x| charset.contains(x)), true);
        }
    }

    #[test]
    fn test_str_charset_macro() {
        let charset = "hello";
        let val = string!(charset);
        for _ in 0..100 {
            let res = build!(val);
            assert_eq!(res.chars().all(|x| charset.contains(x)), true);
        }
    }

    #[test]
    fn test_str_default_macro() {
        let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let val = string!();
        for _ in 0..100 {
            let res = build!(val);
            assert_eq!(res.chars().all(|x| charset.contains(x)), true);
        }
    }

    #[test]
    fn test_int_full_macro() {
        let choices = [3, 4, 5, 6];
        let choices: Vec<String> = choices.iter().map(|x| x.to_string()).collect();
        let val = int!(min = 3, max = 7);
        for _ in 0..100 {
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_int_max_macro() {
        let choices = [0, 1, 2, 3, 4, 5, 6];
        let choices: Vec<String> = choices.iter().map(|x| x.to_string()).collect();
        let val = int!(max = 7);
        for _ in 0..100 {
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_int_default_macro() {
        let choices: Vec<String> = (0..1001).map(|x| x.to_string()).collect();
        let val = int!();
        for _ in 0..1000 {
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }
}
