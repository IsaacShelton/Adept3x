use super::token::PreToken;

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
    Integer,
    Float,
    Character,
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