#[feature(type_alias_impl_trait)]
use crate::rules::Rule;

pub type BoxedBuildable = Box<dyn Buildable>;
pub type BuildableVec = Vec<BoxedBuildable>;
pub type BoxedRuleBuilder<'a> = Box<&'a dyn RuleBuilder>;
pub type OptRuleBuilder<'a> = Option<BoxedRuleBuilder<'a>>;

pub trait RuleBuilder {
    fn get_rule(&self, rule_info: RuleInfo) -> Option<&Rule>;
    fn build_rule(&self, rule_info: RuleInfo) -> String;
    fn build_rule_slow<'a>(&mut self, cat: String, rule_name: String) -> String;
}

pub trait Buildable {
    fn build(&self, output: &mut String, rule_builder: &dyn RuleBuilder);
    fn build_direct(&self, rule_builder: &dyn RuleBuilder) -> String {
        let mut res = String::new();
        self.build(&mut res, rule_builder);
        res
    }
    fn finalize(&mut self, _: &dyn InfoFetcher) {}
    fn is_str(&self) -> bool {
        false
    }
}

impl Buildable for &str {
    fn build(&self, output: &mut String, rule_builder: &dyn RuleBuilder) {
        output.push_str(self);
    }
}

impl Buildable for i32 {
    fn build(&self, output: &mut String, rule_builder: &dyn RuleBuilder) {
        output.push_str(&self.to_string());
    }
}

impl Buildable for f64 {
    fn build(&self, output: &mut String, rule_builder: &dyn RuleBuilder) {
        output.push_str(&self.to_string());
    }
}

/*
impl<T> Buildable for T
where
    T: ToString,
{
    fn build(&self, output: &mut String, rule_builder: &dyn RuleBuilder) {
        output.push_str(&self.to_string());
    }
}
*/

pub struct RuleInfo {
    pub cat_idx: usize,
    pub rule_idx: usize,
}

impl Clone for RuleInfo {
    fn clone(&self) -> Self {
        RuleInfo {
            cat_idx: self.cat_idx,
            rule_idx: self.rule_idx,
        }
    }
}

impl Copy for RuleInfo {}

pub trait InfoFetcher {
    fn get_rule_info(&self, cat: String, rule_name: String) -> Option<RuleInfo>;
}
