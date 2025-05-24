use super::Type;
use attributes::Privacy;
use indexmap::IndexMap;
use num_bigint::BigInt;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct EnumBody<'env> {
    pub backing_type: &'env Type<'env>,
    pub variants: IndexMap<&'env str, EnumVariant>,
    pub privacy: Privacy,
    pub source: Source,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub value: BigInt,
    pub explicit_value: bool,
}
