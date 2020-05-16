#[feature(type_alias_impl_trait)]

pub type BoxedBuildable = Box<dyn Buildable>;
pub type BuildableVec = Vec<BoxedBuildable>;
pub type BoxedRuleBuilder<'a> = Box<&'a dyn RuleBuilder>;
pub type OptRuleBuilder<'a> = Option<BoxedRuleBuilder<'a>>;

pub trait RuleBuilder {
    fn build_rule(&self, cat: String, ref_name: String) -> String;
}

pub trait Buildable {
    fn build(&self, rule_builder: &dyn RuleBuilder) -> String;
}

impl<T> Buildable for T
where
    T: ToString,
{
    fn build(&self, _: &dyn RuleBuilder) -> String {
        self.to_string()
    }
}
