#![macro_use]

use std::boxed::Box;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use super::fields::{Convertible, Item, ItemBuilder, Or};
use super::random::Rand;

pub struct RuleList {
    pub parent: Option<Rc<RefCell<Box<RuleList>>>>,
    pub rules: Vec<Or>,
}

impl RuleList {
    pub fn new() -> RuleList {
        RuleList {
            parent: None,
            rules: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn new_from_parent(
        parent: Option<Rc<RefCell<Box<RuleList>>>>,
    ) -> Rc<RefCell<Box<RuleList>>> {
        let res = Rc::new(RefCell::new(Box::new(RuleList::new())));
        {
            let (res_parent, parent_num_rules) = {
                match parent {
                    Some(v) => {
                        let len = { v.borrow().rules.len() };
                        (Some(v.clone()), len)
                    }
                    None => (None, 0),
                }
            };
            let mut tmp = res.borrow_mut();
            tmp.parent = res_parent;
            for _ in 0..parent_num_rules {
                tmp.add_empty_rule();
            }
        }
        res
    }

    pub fn add_rule<T>(&mut self, rule_idx: usize, rule_value: T)
    where
        T: Convertible,
    {
        self.rules.get_mut(rule_idx).unwrap().add_item(rule_value);
    }

    /// Push a new empty rule into `self.rules`, returning the index of the
    /// new, empty rule
    pub fn add_empty_rule(&mut self) -> usize {
        let res = self.rules.len();
        self.rules.push(Or::new());
        res
    }

    pub fn resolve(&self, rule_idx: usize) -> Result<Option<Rc<RefCell<Box<RuleList>>>>, String> {
        let rule_choices: usize = { self.rules[rule_idx].choices.len() };
        if rule_choices == 0 {
            if self.parent.is_some() {
                let tmp = self.parent.clone().unwrap();
                let parent = tmp.borrow();
                match parent.resolve(rule_idx) {
                    Ok(Some(v)) => Ok(Some(v)),
                    Ok(None) => Ok(self.parent.clone()),
                    Err(v) => Err(v),
                }
            } else {
                Err("No choices in this RuleList, and parent is None".to_string())
            }
        } else {
            Ok(None)
        }
    }

    pub fn get_rule_or(&self, rule_idx: usize) -> &Or {
        &self.rules[rule_idx]
    }
}

// ----------------------------------------------------------------------------

fn add_empty_rule_or<T>(
    rules: &mut RuleList,
    rule_name: T,
    rule_map: &mut BTreeMap<String, usize>,
    rule_map_inv: &mut BTreeMap<usize, String>,
) -> usize
where
    T: Into<String>,
{
    let rule_name = rule_name.into();
    let res = rules.add_empty_rule();
    rule_map.insert(rule_name.clone(), res);
    rule_map_inv.insert(res, rule_name);
    res
}

pub struct RuleSet {
    pub rule_map: BTreeMap<String, usize>,
    pub rule_map_inv: BTreeMap<usize, String>,
    pub rules: Rc<RefCell<Box<RuleList>>>,
    pub rules_root: Rc<RefCell<Box<RuleList>>>,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        let rules = Rc::new(RefCell::new(Box::new(RuleList::new())));
        RuleSet {
            rule_map: BTreeMap::new(),
            rule_map_inv: BTreeMap::new(),
            rules: rules.clone(),
            rules_root: rules,
        }
    }

    pub fn add_rule<T, S>(&mut self, rule_name: S, rule_value: T) -> &mut Self
    where
        S: Into<String>,
        T: Convertible,
    {
        let rule_name = rule_name.into();
        {
            let mut rules = self.rules.borrow_mut();

            let rule_idx = match self.rule_map.get(&rule_name) {
                None => add_empty_rule_or(
                    &mut rules,
                    rule_name,
                    &mut self.rule_map,
                    &mut self.rule_map_inv,
                ),
                Some(v) => *v,
            };
            rules.add_rule(rule_idx, rule_value);
        }
        self
    }

    pub fn finalize(&mut self) {
        loop {
            let mut num_pruned: usize = 0;
            num_pruned += self.finalize_and_prune_rules();
            println!("finalize pruned {}", num_pruned);
            num_pruned += self.calc_shortest_ref_length();
            println!("shortest ref pruned {}", num_pruned);
            if num_pruned == 0 {
                break;
            }
        }
    }

    pub fn finalize_and_prune_rules(&mut self) -> usize {
        println!("Finalizing and pruning rules");
        let mut total_pruned = 0;
        let mut new_rules: HashSet<(usize, String)> = HashSet::new();
        let mut to_prune: HashSet<String> = HashSet::new();
        let mut rules = self.rules.borrow_mut();
        loop {
            new_rules.clear();
            to_prune.clear();
            for (rule_idx, rule_or) in rules.rules.iter_mut().enumerate() {
                let rule_name = &self.rule_map_inv[&rule_idx];
                // has already been pruned
                if !self.rule_map.contains_key(rule_name) {
                    continue;
                }

                let finalized = {
                    let mut fetcher = RefFetcher::new(&self.rule_map);
                    let res = rule_or.finalize(&mut fetcher);
                    for rule_name in fetcher.new_rules.iter() {
                        new_rules.insert((rule_idx, rule_name.clone()));
                    }
                    res
                };

                // rule Or has no options left, everything is unresolvable
                if !finalized && !rule_or.keep {
                    to_prune.insert(rule_name.clone());
                }
            }
            if to_prune.len() == 0 && new_rules.len() == 0 {
                break;
            }
            for (_parent_rule_idx, rule_name) in new_rules.iter() {
                println!("  Adding new rule: {}", rule_name);
                let idx = add_empty_rule_or(
                    &mut rules,
                    rule_name,
                    &mut self.rule_map,
                    &mut self.rule_map_inv,
                );
                rules.rules[idx].keep = true;
            }
            for rule_to_prune in to_prune.iter() {
                println!(
                    "Pruning rule {} due to unresolvable references",
                    rule_to_prune
                );
                self.rule_map.remove(rule_to_prune);
                total_pruned += 1;
            }
        }
        total_pruned
    }

    pub fn calc_shortest_ref_length(&mut self) -> usize {
        println!("Calculating shortest ref lengths");
        let mut rule_lengths: BTreeMap<usize, usize> = BTreeMap::new();
        let mut total_pruned = 0;
        let mut rules = self.rules.borrow_mut();

        loop {
            let mut num_resolved: usize = 0;
            // we only iterate over the rules with resolvable references
            for (_, rule_idx) in self.rule_map.iter() {
                let rule_or = rules.rules.get_mut(*rule_idx).unwrap();

                let num_options_before = rule_or.shortest_options.len();
                let length_calc = RefLenCalculator {
                    rule_lengths: &rule_lengths,
                };
                let new_len = rule_or.calc_ref_length(&length_calc);
                if new_len != 0 {
                    rule_lengths.insert(*rule_idx, new_len);
                }
                let num_options = rule_or.shortest_options.len();
                if num_options > num_options_before {
                    println!(
                        "Resolved {} new options for {}, total {}",
                        num_options - num_options_before,
                        self.rule_map_inv[rule_idx],
                        num_options
                    );
                    num_resolved += 1;
                }
            }
            // there was nothing new that was resolved
            if num_resolved == 0 {
                break;
            }
        }

        for (rule_idx, rule_or) in rules.rules.iter().enumerate() {
            if rule_lengths.contains_key(&rule_idx)
                || !self.rule_map.contains_key(&self.rule_map_inv[&rule_idx])
                || rule_or.keep
            {
                continue;
            }
            println!(
                "Pruning rule {} due to undeterminable reference length",
                self.rule_map_inv[&rule_idx]
            );
            self.rule_map.remove(&self.rule_map_inv[&rule_idx]);
            total_pruned += 1;
        }

        total_pruned
    }

    pub fn get_ref_idx<T>(&self, rule_name: T) -> Option<usize>
    where
        T: Into<String>,
    {
        let rule_name = rule_name.into();
        Some(*self.rule_map.get(&rule_name)?)
    }

    /// Build the rule specified by ref_idx, with output added to `output`,
    /// using `rand`, and the maximum recursion depth of `max_recursion`.
    pub fn build_rule(
        &mut self,
        ref_idx: usize,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        max_recursion: usize,
    ) {
        let builder = ItemBuilder::new(self.rules.clone(), max_recursion);
        builder.build_rule(ref_idx, output, rand, false);

        let mut rules = self.rules.borrow_mut();
        for (rule_idx, new_or) in builder.tmp_rules.borrow_mut().iter_mut() {
            // NOTE: this may be expensive... it should not happen *that* often though.
            // I think. Famous last words probably.
            let rule_or = rules.rules.get_mut(*rule_idx).unwrap();
            for c_item in new_or.choices.iter() {
                let len = rule_or.choices.len();
                // currenly, only static values are ever added dynamically to
                // rules - which makes each of these new rule choices should be
                // in the shortest and available option indices
                rule_or.choice_indices.push(len);
                rule_or.shortest_options.push(len);
                rule_or.choices.push(c_item.to_owned());
            }
            rules.rules[*rule_idx].choices.append(&mut new_or.choices);
        }
    }

    #[allow(dead_code)]
    pub fn build_rule_slow<'a, T>(
        &'a mut self,
        rule_name: T,
        output: &mut Vec<u8>,
        rand: &mut Rand,
        max_recursion: usize,
    ) where
        T: Into<String>,
    {
        let ref_idx = self.get_ref_idx(rule_name).expect("Rule does not exist");
        self.build_rule(ref_idx, output, rand, max_recursion);
    }
}

pub struct RefLenCalculator<'a> {
    rule_lengths: &'a BTreeMap<usize, usize>,
}

impl<'a> RefLenCalculator<'a> {
    pub fn calc_ref_length(&'a self, item: &mut Item) -> usize {
        match item {
            Item::And(v) => v.calc_ref_length(self),
            Item::Or(v) => v.calc_ref_length(self),
            Item::Ref(v) => v.calc_ref_length(self),
            Item::Opt(v) => v.calc_ref_length(self),
            Item::Mul(v) => v.calc_ref_length(self),
            Item::Id(v) => v.calc_ref_length(self),
            _ => 1,
        }
    }

    pub fn get_ref_len(&'a self, rule_idx: usize) -> Option<usize> {
        match self.rule_lengths.get(&rule_idx) {
            Some(v) => Some(*v),
            None => None,
        }
    }
}

pub struct RefFetcher<'a> {
    pub rule_map: &'a BTreeMap<String, usize>,
    pub new_rules: Vec<String>,
}

impl<'a> RefFetcher<'a> {
    pub fn new(rule_map: &'a BTreeMap<String, usize>) -> RefFetcher {
        RefFetcher {
            rule_map,
            new_rules: Vec::new(),
        }
    }

    /// Finalize the `Item`, returning true if the item is fully resolvable
    pub fn finalize(&mut self, item: &mut Item) -> bool {
        match item {
            Item::And(v) => v.finalize(self),
            Item::Ref(v) => v.finalize(self),
            Item::Or(v) => v.finalize(self),
            Item::Opt(v) => v.finalize(self),
            Item::Mul(v) => v.finalize(self),
            Item::Direct(_) => true,
            Item::Str(_) => true,
            Item::Int(_) => true,
            Item::Id(v) => {
                let res = v.finalize(self);
                if !res {
                    println!("  {} did not finalize, adding as new rule", v);
                    println!("    (should finalize next loop)");
                    self.new_rules.push(v.rule_name.clone());
                }
                true
            }
        }
    }

    pub fn get_ref_idx<T>(&'a self, rule_name: T) -> Option<usize>
    where
        T: Into<String>,
    {
        let rule_name = rule_name.into();
        let rule_idx = *self.rule_map.get(&rule_name)?;
        Some(rule_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Rand;
    use std::str;

    #[test]
    fn test_rule_set() {
        let mut rules = RuleSet::new();
        let rules = rules.add_rule("rule", and!(sep = "", "hello", "there"));
        rules.finalize();

        assert_eq!({ rules.rules.borrow().rules.len() }, 1);

        let rule = rules.get_ref_idx("rule");
        assert_eq!(rule.is_some(), true);
    }

    #[test]
    fn test_rule_build() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("rule", and!("hello", "there"))
            .add_rule("rule2", and!("oogah", reff!("rule"), "boogah"));
        let mut rand = Rand::new(0);
        rules.finalize();

        let mut output: Vec<u8> = Vec::new();
        rules.build_rule_slow("rule2", &mut output, &mut rand, 10);
        assert_eq!(
            str::from_utf8(&output[..]).unwrap(),
            "oogahhellothereboogah"
        );
    }

    #[test]
    fn test_auto_prune_normal() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("prune_me", reff!("unresolvable"))
            .add_rule("prune_me2", reff!("prune_me"))
            .add_rule("prune_me3", reff!("prune_me2"))
            .add_rule("rule", "a valid rule")
            .add_rule("rule2", and!("oogah", reff!("rule"), "boogah"));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 2);
        assert_eq!(rules.rule_map.contains_key("rule"), true);
        assert_eq!(rules.rule_map.contains_key("rule2"), true);
        assert_eq!(rules.rule_map.contains_key("prune_me"), false);
        assert_eq!(rules.rule_map.contains_key("prune_me2"), false);
        assert_eq!(rules.rule_map.contains_key("prune_me3"), false);
    }

    #[test]
    fn test_auto_prune_circular() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("prune_me2", reff!("prune_me3"))
            .add_rule("prune_me3", reff!("prune_me2"))
            .add_rule("rule", or!(reff!("prune_me2"), "a valid rule"))
            .add_rule("rule2", and!("oogah", reff!("rule"), "boogah"));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 2);
        assert_eq!(rules.rule_map.contains_key("rule"), true);
        assert_eq!(rules.rule_map.contains_key("rule2"), true);
        assert_eq!(rules.rule_map.contains_key("prune_me2"), false);
        assert_eq!(rules.rule_map.contains_key("prune_me3"), false);
    }

    #[test]
    fn test_auto_prune_circular_or() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("prune_me2", or!(reff!("prune_me3")))
            .add_rule("prune_me3", or!(reff!("prune_me2")))
            .add_rule("rule", or!(reff!("prune_me2"), "a valid rule"))
            .add_rule("rule2", and!("oogah", reff!("rule"), "boogah"));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 2);
        assert_eq!(rules.rule_map.contains_key("rule"), true);
        assert_eq!(rules.rule_map.contains_key("rule2"), true);
        assert_eq!(rules.rule_map.contains_key("prune_me2"), false);
        assert_eq!(rules.rule_map.contains_key("prune_me3"), false);
    }

    #[test]
    fn test_ref_length() {
        let mut rules = RuleSet::new();
        let rules = rules
            .add_rule("rule", and!("rule", reff!("rule1")))
            .add_rule("rule1", and!("rule1", or!("short", reff!("rule2"))))
            .add_rule("rule2", and!("rule2", or!("short", reff!("rule3"))))
            .add_rule("rule3", and!("rule3", or!("short", reff!("rule1"))));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 4);

        let ref_idx = rules.get_ref_idx("rule").unwrap();
        let mut rand = Rand::new(11111);

        let mut max_recursion = 1;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_ne!(res, "rulerule");
        }

        max_recursion = 1;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_eq!(["rulerule1short"].contains(&res), true);
        }

        max_recursion = 2;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                ["rulerule1short", "rulerule1rule2short"].contains(&res),
                true
            );
        }

        max_recursion = 3;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                [
                    "rulerule1short",
                    "rulerule1rule2short",
                    "rulerule1rule2rule3short"
                ]
                .contains(&res),
                true
            );
        }

        max_recursion = 4;
        for _ in 0..100 {
            let mut output: Vec<u8> = Vec::new();
            rules.build_rule(ref_idx, &mut output, &mut rand, max_recursion);
            let res = std::str::from_utf8(&output).unwrap();
            assert_eq!(
                [
                    "rulerule1short",
                    "rulerule1rule2short",
                    "rulerule1rule2rule3short",
                    "rulerule1rule2rule3rule1short"
                ]
                .contains(&res),
                true
            );
        }
    }

    #[test]
    fn test_id() {
        let mut rules = RuleSet::new();
        let rules = rules.add_rule("gen_id", and!("test", id!("new_rule")));
        rules.finalize();

        assert_eq!(rules.rule_map.len(), 2);
        assert_eq!(rules.rule_map.contains_key("gen_id"), true);
        assert_eq!(rules.rule_map.contains_key("new_rule"), true);
        assert_eq!(rules.rules.borrow().rules[1].choices.len(), 0);
    }

    #[test]
    fn test_rule_list() {
        let mut rules = RuleSet::new();
        let rules = rules.add_rule("in_parent", and!("hello", "world"));
        rules.finalize();

        let sub_rules = RuleList::new_from_parent(Some(rules.rules.clone()));
        rules.rules = sub_rules;
        let mut output: Vec<u8> = Vec::new();
        let mut rand: Rand = Rand::new(100);
        rules.build_rule_slow("in_parent", &mut output, &mut rand, 10);

        assert_eq!(str::from_utf8(&output).unwrap(), "helloworld");
    }
}
