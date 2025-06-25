use super::{ConstEvalRef, NodeId, NodeRef, UntypedCfg, cursor::CursorPosition};
use crate::{
    BasicBinaryOperator, ConformBehavior, FillBehavior, Integer, Language, StaticMemberValue, Type,
    UnaryOperator, Using,
};
use arena::Arena;
use attributes::Privacy;
use source_files::Source;
use std::{ffi::CString, fmt::Debug};
use std_ext::SmallVec2;
use token::Name;

#[derive(Clone)]
pub struct Node {
    pub kind: NodeKind,
    pub source: Source,
}

impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

pub fn connect(nodes: &mut Arena<NodeId, Node>, from: CursorPosition, to: NodeRef) {
    let node = &mut nodes[from.from];

    match &mut node.kind {
        NodeKind::Start(next) => {
            assert_eq!(from.edge_index, 0);
            *next = Some(to);
        }
        NodeKind::Sequential(sequential_node) => {
            assert_eq!(from.edge_index, 0);
            sequential_node.next = Some(to);
        }
        NodeKind::Branching(branch) => match from.edge_index {
            0 => branch.when_true = Some(to),
            1 => branch.when_false = Some(to),
            _ => panic!("invalid from edge index for branching node"),
        },
        NodeKind::Terminating(_) => panic!("cannot connect terminating node"),
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SequentialNode {
    pub kind: SequentialNodeKind,
    pub next: Option<NodeRef>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Join {
    pub parent_a: NodeRef,
    pub gives_a: NodeRef,
    pub parent_b: NodeRef,
    pub gives_b: NodeRef,
}

#[derive(Clone, Debug)]
pub enum SequentialNodeKind {
    Join1(NodeRef),
    JoinN(
        SmallVec2<(CursorPosition, NodeRef)>,
        Option<ConformBehavior>,
    ),
    Const(UntypedCfg),
    Name(Name),
    Parameter(String, Type, usize),
    Declare(String, Type, Option<NodeRef>),
    Assign(NodeRef, NodeRef),
    BinOp(NodeRef, BasicBinaryOperator, NodeRef),
    Boolean(bool),
    Integer(Integer),
    Float(f64),
    AsciiChar(u8),
    Utf8Char(String),
    String(String),
    NullTerminatedString(CString),
    Null,
    Void,
    Never,
    Call(Box<NodeCall>),
    DeclareAssign(String, NodeRef),
    Member(NodeRef, String, Privacy),
    ArrayAccess(NodeRef, NodeRef),
    StructLiteral(Box<NodeStructLiteral>),
    UnaryOperation(UnaryOperator, NodeRef),
    StaticMemberValue(Box<StaticMemberValue>),
    StaticMemberCall(Box<NodeStaticMemberCall>),
    SizeOf(Type),
    SizeOfValue(NodeRef),
    InterpreterSyscall(NodeInterpreterSyscall),
    IntegerPromote(NodeRef),
    StaticAssert(ConstEvalRef, Option<String>),
    ConformToBool(NodeRef, Language),
    Is(NodeRef, String),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct NodeCall {
    pub name: Name,
    pub args: Vec<NodeRef>,
    pub expected_to_return: Option<Type>,
    pub generics: Vec<NodeTypeArg>,
    pub using: Vec<Using>,
}

#[derive(Clone, Debug)]
pub enum NodeTypeArg {
    Type(Type),
    Expr(ConstEvalRef),
}

#[derive(Clone, Debug)]
pub struct NodeStaticMemberCall {
    pub subject: Type,
    pub call: NodeCall,
    pub call_source: Source,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct NodeStructLiteral {
    pub ast_type: Type,
    pub fields: Vec<NodeFieldInitializer>,
    pub fill_behavior: FillBehavior,
    pub language: Language,
}

#[derive(Clone, Debug)]
pub struct NodeFieldInitializer {
    pub name: Option<String>,
    pub value: NodeRef,
}

#[derive(Clone, Debug)]
pub struct NodeDeclareAssign {
    pub name: String,
    pub value: NodeRef,
}

#[derive(Clone, Debug)]
pub struct NodeInterpreterSyscall {
    pub kind: interpreter_api::Syscall,
    pub args: Vec<(Type, NodeRef)>,
    pub result_type: Type,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct BranchNode {
    pub condition: NodeRef,
    pub when_true: Option<NodeRef>,
    pub when_false: Option<NodeRef>,
}

#[derive(Clone, Debug)]
pub enum TerminatingNode {
    Return(Option<NodeRef>),
    Computed(Option<NodeRef>),
    Break,
    Continue,
}

#[derive(Clone, Debug)]
pub enum NodeKind {
    Start(Option<NodeRef>),
    Sequential(SequentialNode),
    Branching(BranchNode),
    Terminating(TerminatingNode),
}
