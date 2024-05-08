use super::token::PreToken;
use num_bigint::BigInt;
use num_traits::Zero;

#[derive(Clone, Debug)]
pub struct PreprocessorAst {
    pub group: Group,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub parts: Vec<GroupPart>,
}

#[derive(Clone, Debug)]
pub enum GroupPart {
    IfSection(IfSection),
    ControlLine(ControlLine),
    TextLine(TextLine),
    HashNonDirective,
}

#[derive(Clone, Debug)]
pub struct IfSection {
    pub if_group: IfGroup,
    pub elif_groups: Vec<ElifGroup>,
    pub else_group: Option<Group>,
}

#[derive(Clone, Debug)]
pub enum IfGroup {
    IfLike(IfLike),
    IfDefLike(IfDefLike),
}

#[derive(Clone, Debug)]
pub struct IfLike {
    pub constant_expression: ConstantExpression,
    pub group: Group,
}

#[derive(Clone, Debug)]
pub enum ConstantExpression {
    Ternary,
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
    Cast,
    SizeofExpression,
    SizeofType,
    AlignofType,
    PreIncrement,
    PreDecrement,
    Unary(UnaryOperator),
    ArrayAccess,
    Call,
    Defined,
    MemberViaValue,
    MemberViaPointer,
    PostIncrement,
    PostDecrement,
    CompoundLiteral,
    Identifier,
    Constant(Constant),
    StringLiteral,
    GenericSelection,
}

impl ConstantExpression {
    pub fn is_true(&self) -> bool {
        match self.evaluate() {
            Constant::True => true,
            Constant::Integer(value) => !value.is_zero(),
            Constant::Character(character) => character != '\0',
            _ => false,
        }
    }

    pub fn evaluate(&self) -> Constant {
        match self {
            ConstantExpression::Ternary => {
                unimplemented!("ternary not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::LogicalOr => {
                unimplemented!("logicalOr not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::LogicalAnd => {
                unimplemented!("logicalAnd not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::InclusiveOr => {
                unimplemented!("inclusiveOr not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::ExclusiveOr => {
                unimplemented!("exclusiveOr not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::BitwiseAnd => {
                unimplemented!("bitwiseAnd not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Equals => {
                unimplemented!("equals not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::NotEquals => {
                unimplemented!("notEquals not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::LessThan => {
                unimplemented!("lessThan not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::GreaterThan => {
                unimplemented!("greaterThan not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::LessThanEq => {
                unimplemented!("lessThanEq not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::GreaterThanEq => {
                unimplemented!("greaterThanEq not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::LeftShift => {
                unimplemented!("leftShift not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::RightShift => {
                unimplemented!("rightShift not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Add => {
                unimplemented!("add not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Subtract => {
                unimplemented!("subtract not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Multiply => {
                unimplemented!("multiply not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Divide => {
                unimplemented!("divide not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Modulus => {
                unimplemented!("modulus not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Cast => {
                unimplemented!("cast not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::SizeofExpression => {
                unimplemented!(
                    "sizeofExpression not yet implemented for ConstantExpression::is_true"
                )
            }
            ConstantExpression::SizeofType => {
                unimplemented!("sizeofType not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::AlignofType => {
                unimplemented!("alignofType not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::PreIncrement => {
                unimplemented!("preIncrement not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::PreDecrement => {
                unimplemented!("preDecrement not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Unary(_unary) => {
                unimplemented!("unary not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::ArrayAccess => {
                unimplemented!("arrayAccess not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Call => {
                unimplemented!("call not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Defined => {
                unimplemented!("defined not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::MemberViaValue => {
                unimplemented!("memberViaValue not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::MemberViaPointer => {
                unimplemented!(
                    "memberViaPointer not yet implemented for ConstantExpression::is_true"
                )
            }
            ConstantExpression::PostIncrement => {
                unimplemented!("postIncrement not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::PostDecrement => {
                unimplemented!("postDecrement not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::CompoundLiteral => {
                unimplemented!(
                    "compoundLiteral not yet implemented for ConstantExpression::is_true"
                )
            }
            ConstantExpression::Identifier => {
                unimplemented!("identifier not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::Constant(_constant) => {
                unimplemented!("constant not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::StringLiteral => {
                unimplemented!("stringLiteral not yet implemented for ConstantExpression::is_true")
            }
            ConstantExpression::GenericSelection => {
                unimplemented!(
                    "genericSelection not yet implemented for ConstantExpression::is_true"
                )
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum UnaryOperator {
    AddressOf,
    Dereference,
    Add,
    Subtract,
    BitComplement,
    Not,
}

#[derive(Clone, Debug)]
pub enum Constant {
    Integer(BigInt),
    Float,
    Character(char),
    True,
    False,
    Nullptr,
}

#[derive(Clone, Debug)]
pub enum IfDefKind {
    Defined,
    NotDefined,
}

#[derive(Clone, Debug)]
pub struct IfDefLike {
    pub kind: IfDefKind,
    pub identifier: String,
    pub group: Group,
}

#[derive(Clone, Debug)]
pub enum ElifGroup {
    Elif(IfLike),
    ElifDef(IfDefLike),
}

#[derive(Clone, Debug)]
pub enum ControlLine {
    Include(Vec<PreToken>),
    Embed(Vec<PreToken>),
    Define(Define),
    Undef(String),
    Line(Vec<PreToken>),
    Error(Vec<PreToken>),
    Warning(Vec<PreToken>),
    Pragma(Vec<PreToken>),
}

#[derive(Clone, Debug, Hash)]
pub struct Define {
    pub kind: DefineKind,
    pub name: String,
}

impl Define {
    pub fn overwrites(&self, other: &Define) -> bool {
        match &self.kind {
            DefineKind::Normal(_) => matches!(other.kind, DefineKind::Normal(_)),
            DefineKind::Macro(self_macro) => {
                if let DefineKind::Macro(other_macro) = &other.kind {
                    self_macro.parameters.len() == other_macro.parameters.len()
                        && self_macro.is_variadic == other_macro.is_variadic
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Clone, Debug, Hash)]
pub enum DefineKind {
    Normal(Vec<PreToken>),
    Macro(Macro),
}

#[derive(Clone, Debug, Hash)]
pub struct Macro {
    pub parameters: Vec<String>,
    pub is_variadic: bool,
}

#[derive(Clone, Debug)]
pub struct TextLine {
    pub content: Vec<PreToken>,
}
