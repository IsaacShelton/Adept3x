use super::pre_token::PreToken;
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
    pub tokens: Vec<PreToken>,
    pub group: Group,
}

#[derive(Clone, Debug)]
pub struct Ternary {
    pub condition: ConstExpr,
    pub when_true: ConstExpr,
    pub when_false: ConstExpr,
}

impl Ternary {
    pub fn evaluate(&self) -> i64 {
        if self.condition.is_true() {
            self.when_true.evaluate()
        } else {
            self.when_false.evaluate()
        }
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
}

#[derive(Clone, Debug)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub left: ConstExpr,
    pub right: ConstExpr,
}

impl BinaryOperation {
    pub fn evaluate(&self) -> i64 {
        match self.operator {
            BinaryOperator::LogicalOr => {
                (!self.left.evaluate().is_zero() || !self.right.evaluate().is_zero()) as i64
            }
            BinaryOperator::LogicalAnd => {
                (!self.left.evaluate().is_zero() && !self.right.evaluate().is_zero()) as i64
            }
            BinaryOperator::InclusiveOr => self.left.evaluate() | self.right.evaluate(),
            BinaryOperator::ExclusiveOr => self.left.evaluate() ^ self.right.evaluate(),
            BinaryOperator::BitwiseAnd => self.left.evaluate() & self.right.evaluate(),
            BinaryOperator::Equals => (self.left.evaluate() == self.right.evaluate()) as i64,
            BinaryOperator::NotEquals => (self.left.evaluate() != self.right.evaluate()) as i64,
            BinaryOperator::LessThan => (self.left.evaluate() < self.right.evaluate()) as i64,
            BinaryOperator::GreaterThan => (self.left.evaluate() > self.right.evaluate()) as i64,
            BinaryOperator::LessThanEq => (self.left.evaluate() <= self.right.evaluate()) as i64,
            BinaryOperator::GreaterThanEq => (self.left.evaluate() >= self.right.evaluate()) as i64,
            BinaryOperator::LeftShift => {
                (self
                    .left
                    .evaluate()
                    .overflowing_shl(self.right.evaluate() as u32))
                .0 as i64
            }
            BinaryOperator::RightShift => {
                (self
                    .left
                    .evaluate()
                    .overflowing_shr(self.right.evaluate() as u32))
                .0 as i64
            }
            BinaryOperator::Add => self.left.evaluate().wrapping_add(self.right.evaluate()),
            BinaryOperator::Subtract => self.left.evaluate().wrapping_sub(self.right.evaluate()),
            BinaryOperator::Multiply => self.left.evaluate().wrapping_mul(self.right.evaluate()),
            BinaryOperator::Divide => self.left.evaluate().wrapping_div(self.right.evaluate()),
            BinaryOperator::Modulus => self.left.evaluate().wrapping_rem(self.right.evaluate()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum UnaryOperator {
    Positive,
    Negative,
    BitComplement,
    Not,
}

#[derive(Clone, Debug)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub inner: ConstExpr,
}

impl UnaryOperation {
    pub fn evaluate(&self) -> i64 {
        match self.operator {
            UnaryOperator::Positive => self.inner.evaluate(),
            UnaryOperator::Negative => -self.inner.evaluate(),
            UnaryOperator::BitComplement => !self.inner.evaluate(),
            UnaryOperator::Not => self.inner.evaluate().is_zero() as i64,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ConstExpr {
    Ternary(Box<Ternary>),
    BinaryOperation(Box<BinaryOperation>),
    UnaryOperation(Box<UnaryOperation>),
    Constant(i64),
}

impl ConstExpr {
    pub fn is_true(&self) -> bool {
        !self.evaluate().is_zero()
    }

    pub fn evaluate(&self) -> i64 {
        match self {
            ConstExpr::Ternary(ternary) => ternary.evaluate(),
            ConstExpr::BinaryOperation(binary_operation) => binary_operation.evaluate(),
            ConstExpr::UnaryOperation(unary) => unary.evaluate(),
            ConstExpr::Constant(constant) => *constant,
        }
    }
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
    pub body: Vec<PreToken>,
}

#[derive(Clone, Debug)]
pub struct TextLine {
    pub content: Vec<PreToken>,
}
