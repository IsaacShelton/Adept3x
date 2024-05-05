use super::token::PreToken;

#[derive(Clone, Debug)]
pub struct PreprocessorAst {
    pub group: Group,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub groups: Vec<GroupPart>,
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
    If(IfLike),
    IfDefLike(IfDefLike),
}

#[derive(Clone, Debug)]
pub struct IfLike {
    pub constant_expression: ConstantExpression,
    pub group: Group,
}

#[derive(Clone, Debug)]
pub enum ConstantExpression {}

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

#[derive(Clone, Debug)]
pub struct Define {
    pub kind: DefineKind,
    pub name: String,
}

#[derive(Clone, Debug)]
pub enum DefineKind {
    Normal(Vec<PreToken>),
    Macro(Macro),
}

#[derive(Clone, Debug)]
pub struct Macro {
    pub parameters: Vec<String>,
    pub is_variadic: bool,
}

#[derive(Clone, Debug)]
pub struct TextLine {
    pub content: Vec<PreToken>,
}
