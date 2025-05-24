use ast_workspace::TypeDeclRef;
use derivative::Derivative;
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::{CInteger, FloatSize, IntegerBits, IntegerSign};
use source_files::Source;

#[derive(Clone, Debug, Derivative)]
#[derivative(PartialEq, Eq, Hash)]
pub struct Type<'env> {
    pub kind: TypeKind<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub source: Source,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeArg<'env> {
    Type(&'env Type<'env>),
    Integer(BigInt),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind<'env> {
    // Literals
    IntegerLiteral(BigInt),
    FloatLiteral(Option<NotNan<f64>>),
    // Boolean
    Boolean,
    // Integer
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    SizeInteger(IntegerSign),
    // Floats
    Floating(FloatSize),
    // Pointers
    Ptr(&'env Type<'env>),
    // Void
    Void,
    // Never
    Never,
    // Fixed-Size Array
    FixedArray(&'env Type<'env>, usize),
    // User-Defined
    UserDefined(&'env str, TypeDeclRef, &'env [TypeArg<'env>]),
    // Polymorph
    Polymorph(&'env str),
}
