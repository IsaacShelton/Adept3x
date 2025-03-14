pub mod expr;

use super::token::CToken;
use crate::{
    ast::{Param, Type},
    source_files::Source,
};
use derive_more::{From, IsVariant};
pub use expr::{Expr, Initializer, *};
use itertools::Itertools;

#[derive(Clone, Debug)]
pub struct CTypedef {
    pub ast_type: Type,
}

#[derive(Clone, Debug)]
pub struct TypedefName {
    pub name: String,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct ParameterTypeList {
    pub parameter_declarations: Vec<ParameterDeclaration>,
    pub is_variadic: bool,
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum Abstraction {
    Normal,
    Abstract,
}

#[derive(Clone, Debug)]
pub struct Declarator {
    pub kind: DeclaratorKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum DeclaratorKind {
    Named(String),
    Pointer(Box<Declarator>, Pointer),
    Function(Box<Declarator>, ParameterTypeList),
    Array(Box<Declarator>, ArrayQualifier),
}

impl DeclaratorKind {
    pub fn at(self, source: Source) -> Declarator {
        Declarator { kind: self, source }
    }
}

#[derive(Clone, Debug)]
pub struct AbstractDeclarator {
    pub kind: AbstractDeclaratorKind,
    pub source: Source,
}

#[derive(Clone, Debug, IsVariant)]
pub enum AbstractDeclaratorKind {
    Nothing,
    Pointer(Box<AbstractDeclarator>, Pointer),
    Function(Box<AbstractDeclarator>, ParameterTypeList),
    Array(Box<AbstractDeclarator>, ArrayQualifier),
}

impl AbstractDeclaratorKind {
    pub fn at(self, source: Source) -> AbstractDeclarator {
        AbstractDeclarator { kind: self, source }
    }
}

#[derive(Clone, Debug, IsVariant)]
pub enum ParameterDeclarationCore {
    Declarator(Declarator),
    AbstractDeclarator(AbstractDeclarator),
    Nothing,
}

#[derive(Clone, Debug)]
pub struct ParameterDeclaration {
    pub attributes: Vec<Attribute>,
    pub declaration_specifiers: DeclarationSpecifiers,
    pub core: ParameterDeclarationCore,
    pub source: Source,
}

#[derive(Clone, Debug, Default)]
pub struct Decorators {
    decorators: Vec<Decorator>,
}

#[derive(Clone, Debug)]
pub enum Decorator {
    Pointer(Pointer),
    Array(ArrayQualifier),
    Function(FunctionQualifier),
}

impl Decorator {
    pub fn source(&self) -> Source {
        match self {
            Decorator::Pointer(pointer) => pointer.source,
            Decorator::Array(array) => array.source,
            Decorator::Function(function) => function.source,
        }
    }
}

impl Decorators {
    pub fn then_pointer(&mut self, pointer: Pointer) {
        self.decorators.push(Decorator::Pointer(pointer));
    }

    pub fn then_array(&mut self, array: ArrayQualifier) {
        self.decorators.push(Decorator::Array(array));
    }

    pub fn then_function(&mut self, function: FunctionQualifier) {
        self.decorators.push(Decorator::Function(function));
    }

    pub fn iter(&self) -> impl Iterator<Item = &Decorator> {
        self.decorators.iter()
    }
}

#[derive(Clone, Debug)]
pub struct ArrayQualifier {
    pub expression: Option<Expr>,
    pub type_qualifiers: Vec<TypeQualifier>,
    pub is_static: bool,
    pub is_param_vla: bool,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct FunctionQualifier {
    pub params: Vec<Param>,
    pub is_cstyle_variadic: bool,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Pointer {
    pub attributes: Vec<Attribute>,
    pub type_qualifiers: Vec<TypeQualifier>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct InitDeclarator {
    pub declarator: Declarator,
    pub initializer: Option<Initializer>,
}

#[derive(Clone, Debug)]
pub struct DeclarationSpecifier {
    pub kind: DeclarationSpecifierKind,
    pub source: Source,
}

impl From<TypeSpecifierQualifier> for DeclarationSpecifier {
    fn from(tsq: TypeSpecifierQualifier) -> Self {
        let source = tsq.source();

        DeclarationSpecifier {
            kind: DeclarationSpecifierKind::TypeSpecifierQualifier(tsq),
            source,
        }
    }
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum StorageClassSpecifier {
    Auto,
    Constexpr,
    Extern,
    Register,
    Static,
    ThreadLocal,
    Typedef,
}

impl StorageClassSpecifier {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageClassSpecifier::Auto => "auto",
            StorageClassSpecifier::Constexpr => "constexpr",
            StorageClassSpecifier::Extern => "extern",
            StorageClassSpecifier::Register => "register",
            StorageClassSpecifier::Static => "static",
            StorageClassSpecifier::ThreadLocal => "thread_local",
            StorageClassSpecifier::Typedef => "typedef",
        }
    }
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum FunctionSpecifier {
    Inline,
    Noreturn,
}

impl FunctionSpecifier {
    pub fn as_str(&self) -> &'static str {
        match self {
            FunctionSpecifier::Inline => "inline",
            FunctionSpecifier::Noreturn => "_Noreturn",
        }
    }
}

#[derive(Clone, Debug, From)]
pub enum DeclarationSpecifierKind {
    StorageClassSpecifier(StorageClassSpecifier),
    FunctionSpecifier(FunctionSpecifier),
    TypeSpecifierQualifier(TypeSpecifierQualifier),
}

impl DeclarationSpecifierKind {
    pub fn at(self, source: Source) -> DeclarationSpecifier {
        DeclarationSpecifier { kind: self, source }
    }

    pub fn is_void(&self) -> bool {
        match self {
            Self::TypeSpecifierQualifier(tsq) => tsq.is_void(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeSpecifierQualifier {
    TypeSpecifier(TypeSpecifier),
    TypeQualifier(TypeQualifier),
    AlignmentSpecifier(AlignmentSpecifier),
}

impl TypeSpecifierQualifier {
    pub fn source(&self) -> Source {
        match self {
            TypeSpecifierQualifier::TypeSpecifier(ts) => ts.source,
            TypeSpecifierQualifier::TypeQualifier(tq) => tq.source,
            TypeSpecifierQualifier::AlignmentSpecifier(al) => al.source,
        }
    }

    pub fn is_void(&self) -> bool {
        match self {
            TypeSpecifierQualifier::TypeSpecifier(ts) => ts.kind.is_void(),
            TypeSpecifierQualifier::TypeQualifier(_)
            | TypeSpecifierQualifier::AlignmentSpecifier(_) => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpecifierQualifierList {
    pub attributes: Vec<Attribute>,
    pub type_specifier_qualifiers: Vec<TypeSpecifierQualifier>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct TypeSpecifier {
    pub kind: TypeSpecifierKind,
    pub source: Source,
}

#[derive(Clone, Debug, IsVariant)]
pub enum TypeSpecifierKind {
    Void,
    Bool,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Signed,
    Unsigned,
    Composite(Composite),
    Enumeration(Enumeration),
    TypedefName(TypedefName),
}

#[derive(Clone, Debug)]
pub struct TypeQualifier {
    pub kind: TypeQualifierKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum TypeQualifierKind {
    Const,
    Restrict,
    Volatile,
    Atomic,
}

#[derive(Clone, Debug)]
pub struct AlignmentSpecifier {
    pub kind: AlignmentSpecifierKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum AlignmentSpecifierKind {
    AlignAsType(()),
    AlisnAsConstExpr(()),
}

#[derive(Clone, Debug)]
pub struct DeclarationSpecifiers {
    pub specifiers: Vec<DeclarationSpecifier>,
    pub attributes: Vec<Attribute>,
    pub source: Source,
}

impl From<&SpecifierQualifierList> for DeclarationSpecifiers {
    fn from(value: &SpecifierQualifierList) -> Self {
        let specifiers = value
            .type_specifier_qualifiers
            .iter()
            .map(|tsq| DeclarationSpecifier::from(tsq.clone()))
            .collect_vec();

        Self {
            attributes: value.attributes.clone(),
            specifiers,
            source: value.source,
        }
    }
}

#[derive(Clone, Debug)]
pub enum MemberDeclaration {
    Member(Member),
    StaticAssert(StaticAssertDeclaration),
}

#[derive(Clone, Debug)]
pub struct StaticAssertDeclaration {
    pub condition: ConstExpr,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Member {
    pub attributes: Vec<Attribute>,
    pub specifier_qualifiers: SpecifierQualifierList,
    pub member_declarators: Vec<MemberDeclarator>,
}

#[derive(Clone, Debug)]
pub enum MemberDeclarator {
    Declarator(Declarator),
    BitField(Option<Declarator>, ConstExpr),
}

#[derive(Clone, Debug, From)]
pub enum ExternalDeclaration {
    Declaration(Declaration),
    FunctionDefinition(FunctionDefinition),
}

#[derive(Clone, Debug)]
pub struct BlockItem {
    pub kind: BlockItemKind,
    pub source: Source,
}

#[derive(Clone, Debug, From)]
pub enum BlockItemKind {
    Declaration(Declaration),
    UnlabeledStatement(UnlabeledStatement),
    Label(Label),
}

#[derive(Clone, Debug, From)]
pub enum UnlabeledStatement {
    ExprStatement(ExprStatement),
    PrimaryBlock(Vec<Attribute>, PrimaryBlock),
    JumpStatement(Vec<Attribute>, JumpStatement),
}

#[derive(Clone, Debug)]
pub enum ExprStatement {
    Empty,
    Normal(Vec<Attribute>, Expr),
}

#[derive(Clone, Debug)]
pub struct Attribute {
    pub kind: AttributeKind,
    pub clause: Vec<CToken>,
}

#[derive(Clone, Debug)]
pub enum AttributeKind {
    Standard(String),
    Namespaced(String, String),
}

#[derive(Clone, Debug)]
pub enum JumpStatement {
    Goto(String),
    Continue,
    Break,
    Return(Option<Expr>),
}

#[derive(Clone, Debug)]
pub enum PrimaryBlock {
    CompoundStatement(CompoundStatement),
    SelectionStatement(SelectionStatement),
    IterationStatement(IterationStatement),
}

#[derive(Clone, Debug)]
pub enum IterationStatement {
    While(Expr, SecondaryBlock),
    DoWhile(SecondaryBlock, Expr),
    For(Option<Expr>, Option<Expr>, Option<Expr>),
}

#[derive(Clone, Debug)]
pub struct CompoundStatement {
    pub statements: Vec<BlockItem>,
}

type SecondaryBlock = Statement;

#[derive(Clone, Debug)]
pub enum Statement {
    LabeledStatement(Box<LabeledStatement>),
    UnlabeledStatement(Box<UnlabeledStatement>),
}

#[derive(Clone, Debug)]
pub struct LabeledStatement {
    pub label: Label,
    pub statement: Statement,
}

#[derive(Clone, Debug)]
pub enum SelectionStatement {
    If(Expr, SecondaryBlock),
    IfElse(Expr, SecondaryBlock, SecondaryBlock),
    Switch(Expr, SecondaryBlock),
}

#[derive(Clone, Debug)]
pub enum LabelKind {
    UserDefined(String),
    Case(ConstExpr),
    Default,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub attributes: Vec<Attribute>,
    pub kind: LabelKind,
}

#[derive(Clone, Debug)]
pub enum Declaration {
    Common(CommonDeclaration),
    StaticAssert(StaticAssertDeclaration),
    Attribute(Vec<Attribute>),
}

#[derive(Clone, Debug)]
pub struct CommonDeclaration {
    pub attribute_specifiers: Vec<Attribute>,
    pub declaration_specifiers: DeclarationSpecifiers,
    pub init_declarator_list: Vec<InitDeclarator>,
}

#[derive(Clone, Debug)]
pub struct FunctionDefinition {
    pub attributes: Vec<Attribute>,
    pub declaration_specifiers: DeclarationSpecifiers,
    pub declarator: Declarator,
    pub parameter_type_list: ParameterTypeList,
    pub body: CompoundStatement,
}

#[derive(Clone, Debug)]
pub struct ConstExpr {
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub enum CompositeKind {
    Struct,
    Union,
}

#[derive(Clone, Debug)]
pub struct Composite {
    pub kind: CompositeKind,
    pub source: Source,
    pub name: Option<String>,
    pub attributes: Vec<Attribute>,
    pub members: Option<Vec<MemberDeclaration>>,
}

#[derive(Clone, Debug)]
pub struct EnumTypeSpecifier {
    pub specifier_qualifier_list: SpecifierQualifierList,
}

#[derive(Clone, Debug)]
pub struct Enumerator {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub value: Option<ConstExpr>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum Enumeration {
    Definition(EnumerationDefinition),
    Named(EnumerationNamed),
}

impl Enumeration {
    pub fn source(&self) -> Source {
        match self {
            Enumeration::Definition(definition) => definition.source,
            Enumeration::Named(name) => name.source,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnumerationDefinition {
    pub name: Option<String>,
    pub attributes: Vec<Attribute>,
    pub enum_type_specifier: Option<EnumTypeSpecifier>,
    pub body: Vec<Enumerator>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct EnumerationNamed {
    pub name: String,
    pub enum_type_specifier: Option<EnumTypeSpecifier>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct TypeName {
    pub specifier_qualifiers: SpecifierQualifierList,
    pub abstract_declarator: Option<AbstractDeclarator>,
}
