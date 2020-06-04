#![macro_use]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;
use std::str;

use super::random::Rand;
use super::rules::{RefFetcher, RefLenCalculator};

const SAFE_BUILD: bool = true;

/// Holds the final values that are used to build resulting data
pub enum Item {
    Direct(Vec<u8>),
    And(And),
    Or(Or),
    Ref(Ref),
    Str(Str),
    Int(Int),
    Opt(Opt),
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
            Item::Opt(v) => v.fmt(f),
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
    pub rules: &'a Vec<Vec<(Item, usize)>>,
    pub curr_depth: std::cell::Cell<usize>,
    pub max_depth: usize,
}

impl<'a> ItemBuilder<'a> {
    #[inline]
    pub fn build(&'a self, item: &'a Item, output: &mut Vec<u8>, rand: &mut Rand) {
        match item {
            Item::Direct(v) => self.direct_build(v, output),
            Item::And(v) => v.build(self, output, rand),
            Item::Ref(v) => {
                self.curr_depth.set(self.curr_depth.get() + 1);
                v.build(self, output, rand)
            }
            Item::Or(v) => v.build(self, output, rand, self.curr_depth.get() > self.max_depth),
            Item::Opt(v) => v.build(self, output, rand),
            Item::Str(v) => v.build(self, output, rand),
            Item::Int(v) => v.build(self, output, rand),
        }
    }

    #[inline]
    pub fn fetch_rule(&'a self, rule_idx: usize, rand: &mut Rand) -> Option<&Item> {
        let rules = self.rules.get(rule_idx)?;
        let rand_idx = (rand.next() as usize) % rules.len();
        let (res, _) = rules.get(rand_idx)?;
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
                output.reserve(new_size - old_size);
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
                .map(|x| format!("{}", x))
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

    pub fn finalize(&mut self, fetcher: &RefFetcher) -> bool {
        let mut res = true;
        for item in self.items.iter_mut() {
            res &= fetcher.finalize(item);
        }
        res
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        let mut max_ref_length: usize = 0;
        for item in self.items.iter_mut() {
            let ref_len = length_calc.calc_ref_length(item);
            if ref_len > max_ref_length {
                max_ref_length = ref_len;
            }
        }
        max_ref_length
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
    pub shortest_options: Vec<usize>,
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
                .map(|x| format!("{}", x))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Or {
    pub fn new() -> Or {
        Or {
            choices: Vec::new(),
            shortest_options: Vec::new(),
        }
    }

    #[inline]
    pub fn build(
        &self,
        builder: &ItemBuilder,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        shortest: bool,
    ) {
        let choice_idx = if shortest {
            self.shortest_options[(rand.next() as usize) % self.shortest_options.len()]
        } else {
            (rand.next() as usize) % self.choices.len()
        };
        builder.build(
            self.choices
                .get(choice_idx as usize)
                .expect("Shouldn't fail"),
            output,
            rand,
        );
    }

    pub fn finalize(&mut self, fetcher: &RefFetcher) -> bool {
        let mut to_prune: Vec<usize> = Vec::new();
        for (choice_idx, choice) in self.choices.iter_mut().enumerate() {
            if !fetcher.finalize(choice) {
                to_prune.push(choice_idx);
            }
        }

        // remove from the end of the list first!
        for idx in to_prune.iter().rev() {
            self.choices.remove(*idx);
        }

        // only prune this if we pruned all of our choices first
        self.choices.len() > 0
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        let mut min_ref_length: usize = 0xffffffff;
        let mut ref_lengths: BTreeMap<usize, usize> = BTreeMap::new();

        for (item_idx, item) in self.choices.iter_mut().enumerate() {
            let ref_len = length_calc.calc_ref_length(item);
            ref_lengths.insert(item_idx, ref_len);
            if ref_len < min_ref_length && ref_len != 0 {
                min_ref_length = ref_len;
            }
        }

        for (item_idx, item_len) in ref_lengths.iter() {
            if *item_len == min_ref_length {
                self.shortest_options.push(*item_idx);
            }
        }

        min_ref_length
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
    pub ref_rule: String,
    pub ref_idx: Option<usize>,
}

/// Converts `Ref` to an Item::Ref instance
impl Convertible for Ref {
    fn convert(self) -> Item {
        Item::Ref(self)
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ref<{}>", self.ref_rule)
    }
}

impl Ref {
    pub fn new<T>(ref_rule: T) -> Ref
    where
        T: Into<String>,
    {
        Ref {
            ref_rule: ref_rule.into(),
            ref_idx: None,
        }
    }

    pub fn finalize(&mut self, ref_fetcher: &RefFetcher) -> bool {
        self.ref_idx = ref_fetcher.get_ref_idx(&self.ref_rule);
        self.ref_idx.is_some()
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        let refd_len = match length_calc.get_ref_len(self.ref_idx.unwrap()) {
            Some(v) => v,
            None => return 0,
        };
        refd_len + 1
    }

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) {
        if let None = self.ref_idx {
            panic!(format!(
                "{} was never resolved! Was finalize not called?",
                self
            ));
        }
        let rule = builder
            .fetch_rule(self.ref_idx.unwrap(), rand)
            .expect("Invalid");
        builder.build(&rule, output, rand);
    }
}

#[macro_export]
macro_rules! reff {
    ($ref:expr) => {
        crate::fields::Ref::new($ref)
    };
}

// ----------------------------------------------------------------------------
// Str
// ----------------------------------------------------------------------------

/// The Str struct will be able to create a random string in the range
/// [min, max] using the specified charset
pub struct Str {
    min: usize,
    max: usize,
    diff: usize,
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
    pub fn new<T>(min: usize, max: usize, charset: T) -> Self
    where
        T: Into<Vec<u8>>,
    {
        Str {
            min,
            max,
            diff: max - min,
            charset: charset.into(),
        }
    }

    // no finalize needed

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) {
        let len = ((rand.next() as usize) % self.diff) + self.min;
        let mut res: Vec<u8> = vec![0; len];
        for idx in 0..len {
            let rand_idx = (rand.next() as usize) % self.charset.len();
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
// Opt
// ----------------------------------------------------------------------------

/// The Int struct will be able to create a random i64 in the range
/// [min, max]
pub struct Opt {
    item: Box<Item>,
}

impl Convertible for Opt {
    fn convert(self) -> Item {
        Item::Opt(self)
    }
}

impl fmt::Display for Opt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}?", self.item)
    }
}

impl Opt {
    pub fn new<T: Convertible>(item: T) -> Self {
        Opt {
            item: Box::new(item.convert()),
        }
    }

    pub fn finalize(&mut self, fetcher: &RefFetcher) -> bool {
        fetcher.finalize(&mut self.item)
    }

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) {
        let rand_val = rand.rand_u64(0, 2);
        if rand_val == 0 {
            return;
        }
        builder.build(&self.item, output, rand);
    }
}

#[macro_export]
macro_rules! opt {
    ($item:expr) => {
        crate::fields::Opt::new($item)
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
                rules: &Vec::new(),
                curr_depth: Cell::new(0),
                max_depth: 10,
            };
            let mut rand = Rand::new(since_the_epoch.as_secs());
            let mut tmp_vec: Vec<u8> = Vec::new();
            item_builder.build(&$item.convert(), &mut tmp_vec, &mut rand);
            str::from_utf8(&tmp_vec[..]).unwrap().to_owned()
        }};
        (rand=$rand:expr, $item:expr) => {{
            let item_builder: ItemBuilder = ItemBuilder {
                rules: &Vec::new(),
                curr_depth: Cell::new(0),
                max_depth: 10,
            };
            let mut tmp_vec: Vec<u8> = Vec::new();
            item_builder.build(&$item.convert(), &mut tmp_vec, &mut $rand);
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
        for _ in 0..100 {
            let val = string!(min = 1, max = 5, charset = charset);
            let res = build!(val);
            assert_eq!(res.chars().all(|x| charset.contains(x)), true);
        }
    }

    #[test]
    fn test_str_max_charset_macro() {
        let charset = "hello";
        for _ in 0..100 {
            let val = string!(max = 5, charset = charset);
            let res = build!(val);
            assert_eq!(res.chars().all(|x| charset.contains(x)), true);
        }
    }

    #[test]
    fn test_str_charset_macro() {
        let charset = "hello";
        for _ in 0..100 {
            let val = string!(charset);
            let res = build!(val);
            assert_eq!(res.chars().all(|x| charset.contains(x)), true);
        }
    }

    #[test]
    fn test_str_default_macro() {
        let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        for _ in 0..100 {
            let val = string!();
            let res = build!(val);
            assert_eq!(res.chars().all(|x| charset.contains(x)), true);
        }
    }

    #[test]
    fn test_int_full_macro() {
        let choices = [3, 4, 5, 6];
        let choices: Vec<String> = choices.iter().map(|x| x.to_string()).collect();
        for _ in 0..100 {
            let val = int!(min = 3, max = 7);
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_int_max_macro() {
        let choices = [0, 1, 2, 3, 4, 5, 6];
        let choices: Vec<String> = choices.iter().map(|x| x.to_string()).collect();
        for _ in 0..100 {
            let val = int!(max = 7);
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_int_default_macro() {
        let choices: Vec<String> = (0..1001).map(|x| x.to_string()).collect();
        for _ in 0..1000 {
            let val = int!();
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_opt() {
        let mut build_count = 0;
        let iters = 100;

        let mut rand = Rand::new(100);

        for _ in 0..iters {
            let val = opt!("a");
            let res = build!(rand = rand, val);
            build_count += res.len();
        }
        assert_eq!(0 < build_count && build_count < iters, true);
    }
}
