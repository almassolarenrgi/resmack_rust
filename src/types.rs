#[feature(type_alias_impl_trait)]

pub type BoxedBuildable = Box<dyn Buildable>;
pub type BuildableVec = Vec<BoxedBuildable>;
pub type BoxedRuleBuilder<'a> = Box<&'a dyn RuleBuilder>;
pub type OptRuleBuilder<'a> = Option<BoxedRuleBuilder<'a>>;

pub trait RuleBuilder {
    fn build_rule(&self, rule_info: RuleInfo) -> String;
    fn build_rule_slow<'a>(&mut self, cat: String, rule_name: String) -> String;
}

pub trait Buildable {
    fn build(&self, rule_builder: &dyn RuleBuilder) -> String;
    fn finalize(&mut self, _: &dyn InfoFetcher) {}
}

impl<T> Buildable for T
where
    T: ToString,
{
    fn build(&self, _: &dyn RuleBuilder) -> String {
        self.to_string()
    }
}

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
