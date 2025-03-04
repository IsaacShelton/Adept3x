use super::{AbstractDeclarator, SpecifierQualifierList};
use crate::{
    c::{
        encoding::Encoding,
        token::{FloatSuffix, Integer},
    },
    source_files::Source,
};

#[derive(Clone, Debug)]
pub struct ConstExpr {
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Integer(Integer),
    Float(f64, FloatSuffix),
    StringLiteral(Encoding, String),
    Bool(bool),
    Nullptr,
    Character(Encoding, String),
    Compound(Vec<Expr>),
    BinaryOperation(Box<BinaryOperation>),
    Ternary(Box<Ternary>),
    Cast(Box<Cast>),
    Subscript(Box<Subscript>),
    Field(Box<Field>),
    PostIncrement(Box<Expr>),
    PostDecrement(Box<Expr>),
    Identifier(String),
    EnumConstant(String, Integer),
    CompoundLiteral(Box<CompoundLiteral>),
    AddressOf(Box<Expr>),
    Dereference(Box<Expr>),
    Negate(Box<Expr>),
    BitComplement(Box<Expr>),
    Not(Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
}

impl ExprKind {
    pub fn at(self, source: Source) -> Expr {
        Expr { kind: self, source }
    }
}

#[derive(Clone, Debug)]
pub enum BinaryOperator {
    LogicalOr,
    LogicalAnd,
    InclusiveOr,
    ExclusiveOr,
    BitwiseAnd,
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessThanEq,
    GreaterThanEq,
    LeftShift,
    RightShift,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModulusAssign,
    LeftShiftAssign,
    RightShiftAssign,
    BitAndAssign,
    BitXorAssign,
    BitOrAssign,
}

#[derive(Clone, Debug)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub left: Expr,
    pub right: Expr,
}

#[derive(Clone, Debug)]
pub struct Ternary {
    pub condition: Expr,
    pub when_true: Expr,
    pub when_false: Expr,
}

#[derive(Clone, Debug)]
pub struct Cast {
    pub specializer_qualifiers: SpecifierQualifierList,
    pub abstract_declarator: Option<AbstractDeclarator>,
    pub inner: Expr,
}

#[derive(Clone, Debug)]
pub struct Subscript {
    pub subject: Expr,
    pub subscript: Expr,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub subject: Expr,
    pub field: String,
    pub source: Source,
    pub is_pointer: bool,
}

#[derive(Clone, Debug)]
pub struct Caster {
    pub specializer_qualifiers: SpecifierQualifierList,
    pub abstract_declarator: Option<AbstractDeclarator>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct CompoundLiteral {
    pub caster: Caster,
    pub braced_initializer: BracedInitializer,
}

#[derive(Clone, Debug)]
pub struct BracedInitializer {
    pub designated_initializers: Vec<DesignatedInitializer>,
}

#[derive(Clone, Debug)]
pub struct DesignatedInitializer {
    pub designation: Option<Designation>,
    pub initializer: Initializer,
}

#[derive(Clone, Debug)]
pub enum Initializer {
    Expression(Expr),
    BracedInitializer(BracedInitializer),
}

#[derive(Clone, Debug)]
pub struct Designation {
    pub path: Vec<Designator>,
}

#[derive(Clone, Debug)]
pub enum Designator {
    Subscript(ConstExpr),
    Field(String),
}
