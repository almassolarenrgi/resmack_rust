#![macro_use]

use std::boxed::Box;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;
use std::str;

use super::random::Rand;
use super::rules::{RefFetcher, RefLenCalculator, RuleList};

const SAFE_BUILD: bool = true;

/// Holds the final values that are used to build resulting data
#[derive(Clone)]
pub enum Item {
    Direct(Vec<u8>),
    And(And),
    Or(Or),
    Ref(Ref),
    Str(Str),
    Int(Int),
    Opt(Opt),
    Mul(Mul),
    Id(Id),
    PreId(PreId),
    PreFlush,
    Scoped(Scoped),
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Direct(v) => write!(
                f,
                "u8[{}]",
                std::str::from_utf8(
                    &v.iter()
                        .map(|b| std::ascii::escape_default(*b))
                        .flatten()
                        .collect::<Vec<u8>>()
                )
                .unwrap()
            ),
            Item::And(v) => v.fmt(f),
            Item::Or(v) => v.fmt(f),
            Item::Ref(v) => v.fmt(f),
            Item::Str(v) => v.fmt(f),
            Item::Int(v) => v.fmt(f),
            Item::Opt(v) => v.fmt(f),
            Item::Mul(v) => v.fmt(f),
            Item::Id(v) => v.fmt(f),
            Item::PreId(v) => v.fmt(f),
            Item::PreFlush => write!(f, "PreFlush"),
            Item::Scoped(v) => v.fmt(f),
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
impl<'a> Convertible for Item {
    #[inline]
    fn convert(self) -> Item {
        self
    }
}

/// Converts `String` to an Item::Direct instance
impl<'a> Convertible for &str {
    #[inline]
    fn convert(self) -> Item {
        Item::Direct(self.as_bytes().to_vec())
    }
}

/// Converts `String` to an Item::Direct instance
impl<'a> Convertible for &[u8] {
    #[inline]
    fn convert(self) -> Item {
        Item::Direct(self.to_vec())
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

pub struct ItemBuilder {
    pub rules: Rc<RefCell<Box<RuleList>>>,
    pub curr_depth: Cell<usize>,
    pub max_depth: usize,
}

impl ItemBuilder {
    pub fn new(rules: Rc<RefCell<Box<RuleList>>>, max_depth: usize) -> ItemBuilder {
        ItemBuilder {
            rules: rules,
            curr_depth: Cell::new(0),
            max_depth,
        }
    }

    #[inline]
    pub fn build(
        &self,
        item: &Item,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
    ) {
        let shortest = self.curr_depth.get() >= self.max_depth;
        match item {
            Item::Direct(v) => self.direct_build(v, output),
            Item::And(v) => v.build(self, pre_output, output, rand),
            Item::Ref(v) => {
                self.curr_depth.set(self.curr_depth.get() + 1);
                v.build(self, pre_output, output, rand, shortest);
                self.curr_depth.set(self.curr_depth.get() - 1);
            }
            Item::Or(v) => v.build(self, pre_output, output, rand, shortest),
            Item::Opt(v) => v.build(self, pre_output, output, rand, shortest),
            Item::Str(v) => v.build(self, pre_output, output, rand),
            Item::Int(v) => v.build(self, pre_output, output, rand),
            Item::Mul(v) => v.build(self, pre_output, output, rand),
            Item::Id(v) => {
                let built_id = v.build(self, output, rand);
                let rule_idx = v.rule_idx.unwrap();
                self.rules.borrow().rules[rule_idx]
                    .borrow_mut()
                    .add_item(built_id);
            }
            Item::PreId(v) => {
                let built_id = v.build(self, pre_output, output, rand);
                let rule_idx = v.rule_idx.unwrap();
                self.rules.borrow().rules[rule_idx]
                    .borrow_mut()
                    .add_item(built_id);
            }
            Item::PreFlush => {
                pre_output.extend(&output[..]);
                output.clear();
            }
            Item::Scoped(v) => {
                let scoped_rules = RuleList::new_from_parent(Some(self.rules.clone()));
                let new_builder = ItemBuilder::new(scoped_rules, self.max_depth);
                new_builder.curr_depth.set(self.curr_depth.get());
                v.build(&new_builder, pre_output, output, rand);
                // all scoped rules added by id!() are discarded - DO NOT MERGE
                // THEM INTO THIS ITEMBUILDER
            }
        }
    }

    #[inline]
    pub fn build_rule(
        &self,
        rule_idx: usize,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        shortest: bool,
    ) {
        let mut rules = self.rules.clone();
        let mut options: Vec<Rc<RefCell<Box<RuleList>>>> = Vec::new();
        loop {
            rules = {
                let rules_b = rules.borrow();
                if rules_b.rules[rule_idx].borrow().choice_indices.len() > 0 {
                    options.push(rules.clone());
                }
                if rules_b.parent.is_none() {
                    break;
                }
                rules_b.parent.as_ref().unwrap().clone()
            };
        }
        if options.len() == 0 {
            panic!(format!("Could not build rule for rule_idx {}", rule_idx));
        }
        let rand_idx: usize = rand.next() as usize % options.len();
        #[rustfmt::skip]
        let rules = options[rand_idx].borrow();
        rules.rules[rule_idx]
            .borrow()
            .build(self, pre_output, output, rand, shortest);
    }

    #[inline]
    pub fn direct_build(&self, v: &Vec<u8>, output: &mut Vec<u8>) {
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

#[derive(Clone)]
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

    pub fn finalize(&mut self, fetcher: &mut RefFetcher) -> bool {
        let mut res = true;
        for item in self.items.iter_mut() {
            res &= fetcher.finalize(item);
        }
        res
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        let mut max_ref_length: usize = 0;
        let mut all_resolved = true;
        for item in self.items.iter_mut() {
            let ref_len = length_calc.calc_ref_length(item);
            if ref_len == 0 {
                all_resolved = false;
            } else if ref_len > max_ref_length {
                max_ref_length = ref_len;
            }
        }
        if all_resolved {
            max_ref_length
        } else {
            0
        }
    }

    #[inline]
    pub fn build(
        &self,
        builder: &ItemBuilder,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
    ) {
        for (idx, item) in self.items.iter().enumerate() {
            if self.sep.len() > 0 && idx > 0 {
                builder.direct_build(&self.sep, output);
            }
            builder.build(item, pre_output, output, rand);
        }
    }
}

#[macro_export]
macro_rules! and {
    (sep = $sep:expr, $($item:expr),* $(,)?) => {
        $crate::fields::And::new($sep)
            $(.add_item($item))*
    };
    ($($item:expr),* $(,)?) => {
        $crate::fields::And::new("")
            $(.add_item($item))*
    };
}

// ----------------------------------------------------------------------------
// OR
// ----------------------------------------------------------------------------

#[derive(Clone)]
pub struct Or {
    pub choices: Vec<Item>,
    pub shortest_options: Vec<usize>,
    pub choice_indices: Vec<usize>,
    pub keep: bool,
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
            "Or<keep={:?} {}>",
            self.keep,
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
            choice_indices: Vec::new(),
            shortest_options: Vec::new(),
            keep: false,
        }
    }

    pub fn new_keep() -> Or {
        Or {
            choices: Vec::new(),
            choice_indices: Vec::new(),
            shortest_options: Vec::new(),
            keep: true,
        }
    }

    #[inline]
    pub fn build(
        &self,
        builder: &ItemBuilder,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        shortest: bool,
    ) {
        let choice = self.get_item(rand, shortest);
        builder.build(choice, pre_output, output, rand);
    }

    pub fn finalize(&mut self, fetcher: &mut RefFetcher) -> bool {
        self.choice_indices.clear();

        for (choice_idx, choice) in self.choices.iter_mut().enumerate() {
            if fetcher.finalize(choice) {
                self.choice_indices.push(choice_idx);
            }
        }

        // only prune this if we pruned all of our choices first
        self.choice_indices.len() > 0
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        self.shortest_options.clear();

        let mut min_ref_length: usize = std::usize::MAX;
        let mut ref_lengths: BTreeMap<usize, usize> = BTreeMap::new();

        for item_idx in self.choice_indices.iter_mut() {
            let item = self.choices.get_mut(*item_idx).unwrap();
            // these should *NEVER* be used in shortest=true build situations
            //if let Item::PreId(_) = item {
            //    continue;
            //}
            let ref_len = length_calc.calc_ref_length(item);
            ref_lengths.insert(*item_idx, ref_len);
            if ref_len < min_ref_length && ref_len != 0 {
                min_ref_length = ref_len;
            }
        }

        for (item_idx, item_len) in ref_lengths.iter() {
            if *item_len == min_ref_length {
                self.shortest_options.push(*item_idx);
            }
        }

        if min_ref_length == std::usize::MAX {
            min_ref_length = 0;
        }

        if self.keep && min_ref_length == 0 {
            min_ref_length = 1
        }

        min_ref_length
    }

    #[inline]
    pub fn get_item(&self, rand: &mut Rand, shortest: bool) -> &Item {
        // keep = true means that this is a dynamic Or, which means that all
        // items will be Item::Direct
        let choice_idx = if shortest && self.shortest_options.len() > 0 {
            self.shortest_options[(rand.next() as usize) % self.shortest_options.len()]
        } else {
            self.choice_indices[(rand.next() as usize) % self.choice_indices.len()]
        };
        &self.choices[choice_idx]
    }

    pub fn add_item<T: Convertible>(&mut self, choice: T) -> &Self {
        self.choice_indices.push(self.choices.len());
        self.choices.push(choice.convert());
        self
    }

    pub fn print_options(&self, shortest: bool, prefix: &str) {
        let print_opt = |opt| println!("{}{}", prefix, opt);

        println!("{}keep: {:?}", prefix, self.keep);
        println!("{}shortest: {:?}", prefix, shortest);

        if !shortest {
            for opt_idx in self.choice_indices.iter() {
                print_opt(&self.choices[*opt_idx]);
            }
        } else {
            for opt_idx in self.shortest_options.iter() {
                print_opt(&self.choices[*opt_idx]);
            }
        }
    }
}

#[macro_export]
macro_rules! or {
    ($($item:expr),* $(,)?) => {
        {
            let mut tmp = $crate::fields::Or::new();
            $(tmp.add_item($item);)*
            tmp
        }
    }
}

// ----------------------------------------------------------------------------
// Ref
// ----------------------------------------------------------------------------

#[derive(Clone)]
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

    #[allow(dead_code)]
    pub fn new_with_idx<T>(ref_rule: T, idx: usize) -> Ref
    where
        T: Into<String>,
    {
        Ref {
            ref_rule: ref_rule.into(),
            ref_idx: Some(idx),
        }
    }

    pub fn finalize(&mut self, ref_fetcher: &mut RefFetcher) -> bool {
        self.ref_idx = ref_fetcher.get_ref_idx(&self.ref_rule);
        self.ref_idx.is_some()
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        let ref_idx = match self.ref_idx {
            Some(v) => v,
            None => panic!(format!("Rule {:?} was never resolved", self.ref_rule)),
        };
        let refd_len = match length_calc.get_ref_len(ref_idx) {
            Some(v) => v,
            None => 0,
        };
        if refd_len == 0 {
            return 0;
        }
        refd_len + 1
    }

    #[inline]
    pub fn build(
        &self,
        builder: &ItemBuilder,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        shortest: bool,
    ) {
        if let None = self.ref_idx {
            panic!(format!(
                "{} was never resolved! Was finalize not called?",
                self
            ));
        }
        builder.build_rule(self.ref_idx.unwrap(), pre_output, output, rand, shortest);
    }
}

#[macro_export]
macro_rules! reff {
    ($ref:expr) => {
        $crate::fields::Ref::new($ref)
    };
}

// ----------------------------------------------------------------------------
// Str
// ----------------------------------------------------------------------------

/// The Str struct will be able to create a random string in the range
/// [min, max] using the specified charset
#[derive(Clone)]
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
    pub fn build(
        &self,
        builder: &ItemBuilder,
        _pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
    ) {
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
        $crate::fields::Str::new($min, $max, $charset)
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
#[derive(Clone)]
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
    pub fn build(
        &self,
        builder: &ItemBuilder,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
    ) {
        let val = rand.rand_i64(self.min, self.max);
        builder.direct_build(&val.to_string().as_bytes().to_vec(), output);
    }
}

#[macro_export]
macro_rules! int {
    (min = $min:expr, max = $max:expr) => {
        $crate::fields::Int::new($min, $max)
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
#[derive(Clone)]
pub struct Opt {
    item: Box<Item>,
    has_refs: bool,
}

impl Convertible for Opt {
    fn convert(self) -> Item {
        Item::Opt(self)
    }
}

impl fmt::Display for Opt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Opt<{}>", self.item)
    }
}

impl Opt {
    #[allow(dead_code)]
    pub fn new<T: Convertible>(item: T) -> Self {
        Opt {
            item: Box::new(item.convert()),
            has_refs: false,
        }
    }

    pub fn finalize(&mut self, fetcher: &mut RefFetcher) -> bool {
        fetcher.finalize(&mut self.item)
    }

    #[inline]
    pub fn build(
        &self,
        builder: &ItemBuilder,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        shortest: bool,
    ) {
        if shortest && self.has_refs {
            return;
        }
        let rand_val = rand.rand_u64(0, 2);
        if rand_val == 0 {
            return;
        }
        builder.build(&self.item, pre_output, output, rand);
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        let res = length_calc.calc_ref_length(&mut self.item);
        if res > 1 {
            self.has_refs = true;
        }
        res
    }
}

#[macro_export]
macro_rules! opt {
    ($item:expr) => {
        $crate::fields::Opt::new($item)
    };
}

// ----------------------------------------------------------------------------
// Mul
// ----------------------------------------------------------------------------

/// The Mul struct handles both `star!` and `plus!` macros
#[derive(Clone)]
pub struct Mul {
    item: Box<Item>,
    min: usize,
    max: usize,
}

impl Convertible for Mul {
    fn convert(self) -> Item {
        Item::Mul(self)
    }
}

impl fmt::Display for Mul {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<Mul {{[{},{})}} {}>", self.min, self.max, self.item)
    }
}

impl Mul {
    #[allow(dead_code)]
    pub fn new<T: Convertible>(item: T, min: usize, max: usize) -> Self {
        if max < min {
            panic!("Mul: max must be greater than min");
        }
        Mul {
            item: Box::new(item.convert()),
            min,
            max,
        }
    }

    pub fn finalize(&mut self, fetcher: &mut RefFetcher) -> bool {
        fetcher.finalize(&mut self.item)
    }

    #[inline]
    pub fn build(
        &self,
        builder: &ItemBuilder,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
    ) {
        let num_times = if self.max > self.min {
            rand.rand_usize(self.min, self.max)
        } else {
            self.min
        };
        for _ in 0..num_times {
            builder.build(&self.item, pre_output, output, rand);
        }
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        length_calc.calc_ref_length(&mut self.item)
    }
}

#[macro_export]
macro_rules! star {
    (min=$min:expr, max=$max:expr, $item:expr) => {
        $crate::fields::Mul::new($item, $min, $max)
    };
    (max=$max:expr, $item:expr) => {
        $crate::fields::Mul::new($item, 0, $max)
    };
    ($item:expr) => {
        $crate::fields::Mul::new($item, 0, 10)
    };
}

#[macro_export]
macro_rules! plus {
    (max=$max:expr, $item:expr) => {
        star!(min = 1, max = $max, $item)
    };
    ($item:expr) => {
        star!(min = 1, max = 10, $item)
    };
}

// ----------------------------------------------------------------------------
// Id
// ----------------------------------------------------------------------------

/// The Id struct is responsible for generating a new identifier, writing
/// the value into the output, and adding the identifier as an option of the
/// specified rule name in the grammar.
#[derive(Clone)]
pub struct Id {
    pub rule_name: String,
    rule_idx: Option<usize>,
}

impl Convertible for Id {
    fn convert(self) -> Item {
        Item::Id(self)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Id<{}>", self.rule_name)
    }
}

impl Id {
    #[allow(dead_code)]
    pub fn new<T>(rule_name: T) -> Self
    where
        T: Into<String>,
    {
        Id {
            rule_name: rule_name.into(),
            rule_idx: None,
        }
    }

    pub fn finalize(&mut self, fetcher: &mut RefFetcher) -> bool {
        self.rule_idx = fetcher.get_ref_idx(&self.rule_name);
        self.rule_idx.is_some()
    }

    #[inline]
    pub fn build(&self, builder: &ItemBuilder, output: &mut Vec<u8>, rand: &mut Rand) -> String {
        // [10-20)
        let rand_len = ((rand.next() as usize) % 10) + 10;
        let mut res: Vec<u8> = vec![0; rand_len];
        let charset = "abcdefghijklmnopqrstuvwxyz".as_bytes();
        let len = charset.len();
        for idx in 0..rand_len {
            let rand_idx = (rand.next() as usize) % len;
            res[idx] = charset[rand_idx];
        }
        builder.direct_build(&res, output);
        String::from_utf8(res).expect("Invalid UTF8 somehow")
    }

    pub fn calc_ref_length(&mut self, _length_calc: &RefLenCalculator) -> usize {
        1
    }
}

/// The `id!("rule_name")` macro creates a new random value that is stored as
/// an option for `rule_name` *and* is written to the output buffer. Values
/// generated by `id!("rule_name")` are accessible immediately after they are
/// created.
///
/// **HOWEVER**, it is up to the user to ensure that the destination rule of the
/// `id!()` is valid at the time of reference. Eventually resmack may address
/// this.
#[macro_export]
macro_rules! id {
    ($rule_name:expr) => {
        $crate::fields::Id::new($rule_name)
    };
}

// ----------------------------------------------------------------------------
// PreId
// ----------------------------------------------------------------------------

pub const PRE_ID: &[u8] = b"%PREID";

/// The Id struct is responsible for generating a new identifier, writing
/// the value into the output, and adding the identifier as an option of the
/// specified rule name in the grammar.
#[derive(Clone)]
pub struct PreId {
    pub rule_name: String,
    rule_idx: Option<usize>,
    sep: Vec<u8>,
    items: Vec<Item>,
    id_indices: Vec<usize>,
}

impl Convertible for PreId {
    fn convert(self) -> Item {
        Item::PreId(self)
    }
}

impl fmt::Display for PreId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PreId<rule={}>", self.rule_name)
    }
}

impl PreId {
    #[allow(dead_code)]
    pub fn new<T, S>(rule_name: T, sep: S) -> Self
    where
        T: Into<String>,
        S: Convertible,
    {
        PreId {
            rule_name: rule_name.into(),
            rule_idx: None,
            sep: match sep.convert() {
                Item::Direct(v) => v,
                _ => panic!("Separator may only be an Item::Direct"),
            },
            items: Vec::new(),
            id_indices: Vec::new(),
        }
    }

    pub fn add_item<T: Convertible>(mut self, item: T) -> Self {
        self.items.push(item.convert());
        self
    }

    /// Returns a tuple of bools: `(items_finalized, ref_finalized)`
    pub fn finalize(&mut self, fetcher: &mut RefFetcher) -> (bool, bool) {
        let mut res = true;
        for (idx, item) in self.items.iter_mut().enumerate() {
            let item_res = fetcher.finalize(item);
            res &= item_res;
            if let Item::Direct(v) = item {
                if v == &PRE_ID {
                    self.id_indices.push(idx);
                }
            }
        }

        self.rule_idx = fetcher.get_ref_idx(&self.rule_name);
        (res, self.rule_idx.is_some())
    }

    pub fn calc_ref_length(&mut self, length_calc: &RefLenCalculator) -> usize {
        let mut max_ref_length: usize = 0;
        let mut all_resolved = true;
        for item in self.items.iter_mut() {
            let ref_len = length_calc.calc_ref_length(item);
            if ref_len == 0 {
                all_resolved = false;
            } else if ref_len > max_ref_length {
                max_ref_length = ref_len;
            }
        }
        if all_resolved {
            // should always be one more than what the items say. PreId is
            // basically a double-ref
            max_ref_length
        } else {
            0
        }
    }

    #[inline]
    pub fn build(
        &self,
        builder: &ItemBuilder,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
    ) -> String {
        // [10-20)
        let rand_len = ((rand.next() as usize) % 10) + 10;
        let mut res: Vec<u8> = vec![0; rand_len];
        let charset = "abcdefghijklmnopqrstuvwxyz".as_bytes();
        let len = charset.len();
        for idx in 0..rand_len {
            let rand_idx = (rand.next() as usize) % len;
            res[idx] = charset[rand_idx];
        }

        // do the pre-build
        let mut tmp_output: Vec<u8> = Vec::new();
        for (idx, item) in self.items.iter().enumerate() {
            if self.sep.len() > 0 && idx > 0 {
                builder.direct_build(&self.sep, output);
            }
            if self.id_indices.contains(&idx) {
                builder.direct_build(&res, &mut tmp_output);
            } else {
                builder.build(item, pre_output, &mut tmp_output, rand);
            }
        }
        pre_output.extend(&tmp_output);

        builder.direct_build(&res, output);
        String::from_utf8(res).expect("Invalid UTF8 somehow")
    }
}

/// The `pre_id!(rule="rule_name", sep=")` macro creates a new random value that is stored as
/// an option for `rule_name` *and* is written to the output buffer. Values
/// generated by `id!("rule_name")` are accessible immediately after they are
/// created.
///
/// **HOWEVER**, it is up to the user to ensure that the destination rule of the
/// `id!()` is valid at the time of reference. Eventually resmack may address
/// this.
#[macro_export]
macro_rules! pre_id {
    (rule=$rule_name:expr, sep=$sep:expr, $($item:expr),* $(,)?) => {
        $crate::fields::PreId::new($rule_name, $sep)
            $(.add_item($item))*
    };
}

/// The `pre_flush!()` macro causes the current pre_output and output to be
/// concatenated together. If not manually performed, this will only occur
/// at the end of a top-level `build_rule()` call on a `RuleSet` instance.
#[macro_export]
macro_rules! pre_flush {
    () => {
        $crate::fields::Item::PreFlush
    };
}

// ----------------------------------------------------------------------------
// Scope
// ----------------------------------------------------------------------------

/// The Scoped struct creates a new scoped rule set that will be discarded
/// when this scope's item is done being generated. All `id!()` values
/// will remain in this scope and not be available outside of the scope.
#[derive(Clone)]
pub struct Scoped {
    item: Box<Item>,
}

impl Convertible for Scoped {
    fn convert(self) -> Item {
        Item::Scoped(self)
    }
}

impl fmt::Display for Scoped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Scoped<{}>", self.item)
    }
}

impl Scoped {
    pub fn new<T: Convertible>(item: T) -> Self {
        Scoped {
            item: Box::new(item.convert()),
        }
    }

    pub fn finalize(&mut self, fetcher: &mut RefFetcher) -> bool {
        fetcher.finalize(&mut self.item)
    }

    #[inline]
    pub fn build(
        &self,
        builder: &ItemBuilder,
        pre_output: &mut Vec<u8>,
        output: &mut Vec<u8>,
        rand: &mut Rand,
    ) {
        builder.build(&self.item, pre_output, output, rand);
    }
}

#[macro_export]
macro_rules! scoped {
    ($item:expr) => {
        $crate::fields::Scoped::new($item)
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
            let rules = Rc::new(RefCell::new(Box::new(RuleList::new())));
            let item_builder: ItemBuilder = ItemBuilder::new(rules, 10);
            let mut rand = Rand::new(since_the_epoch.as_secs());
            let mut tmp_pre_vec: Vec<u8> = Vec::new();
            let mut tmp_vec: Vec<u8> = Vec::new();
            item_builder.build(&$item.convert(), &mut tmp_pre_vec, &mut tmp_vec, &mut rand);
            tmp_pre_vec.extend(&tmp_vec);
            str::from_utf8(&tmp_pre_vec[..]).unwrap().to_owned()
        }};
        (rand=$rand:expr, $item:expr) => {{
            let rules = Rc::new(RefCell::new(Box::new(RuleList::new())));
            let item_builder: ItemBuilder = ItemBuilder::new(rules, 10);
            let mut tmp_pre_vec: Vec<u8> = Vec::new();
            let mut tmp_vec: Vec<u8> = Vec::new();
            item_builder.build(&$item.convert(), &mut tmp_pre_vec, &mut tmp_vec, &mut $rand);
            tmp_pre_vec.extend(&tmp_vec);
            str::from_utf8(&tmp_pre_vec[..]).unwrap().to_owned()
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

    #[test]
    fn test_star_default() {
        let choices: Vec<String> = (0..10)
            .map(|x| {
                let mut tmp = String::new();
                for _ in 0..x {
                    tmp.push_str("hello");
                }
                tmp.clone()
            })
            .collect();
        for _ in 0..1000 {
            let val = star!("hello");
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_star_max() {
        let max = 7;
        let choices: Vec<String> = (0..max)
            .map(|x| {
                let mut tmp = String::new();
                for _ in 0..x {
                    tmp.push_str("hello");
                }
                tmp.clone()
            })
            .collect();
        for _ in 0..1000 {
            let val = star!(max = max, "hello");
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_star_max_min() {
        let min = 4;
        let max = 7;
        let choices: Vec<String> = (min..max)
            .map(|x| {
                let mut tmp = String::new();
                for _ in 0..x {
                    tmp.push_str("hello");
                }
                tmp.clone()
            })
            .collect();
        for _ in 0..1000 {
            let val = star!(min = min, max = max, "hello");
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_plus_max() {
        let max = 7;
        let choices: Vec<String> = (1..max)
            .map(|x| {
                let mut tmp = String::new();
                for _ in 0..x {
                    tmp.push_str("hello");
                }
                tmp.clone()
            })
            .collect();
        for _ in 0..1000 {
            let val = plus!(max = max, "hello");
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }

    #[test]
    fn test_plus_default() {
        let min = 1;
        let max = 10;
        let choices: Vec<String> = (min..max)
            .map(|x| {
                let mut tmp = String::new();
                for _ in 0..x {
                    tmp.push_str("hello");
                }
                tmp.clone()
            })
            .collect();
        for _ in 0..1000 {
            let val = plus!("hello");
            let res = build!(val);
            assert_eq!(choices.contains(&res), true);
        }
    }
}
