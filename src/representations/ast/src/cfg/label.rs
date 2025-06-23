use super::{BranchNode, Node, NodeKind, SequentialNodeKind, TerminatingNode};
use std::borrow::Cow;

pub trait Label {
    fn label(&self) -> Cow<str>;
}

impl Label for Node {
    fn label(&self) -> Cow<str> {
        self.kind.label()
    }
}

impl Label for NodeKind {
    fn label(&self) -> Cow<str> {
        match self {
            NodeKind::Start(_) => "start".into(),
            NodeKind::Sequential(sequential_node) => sequential_node.kind.label(),
            NodeKind::Branching(branch_node) => branch_node.label(),
            NodeKind::Terminating(terminating_node) => terminating_node.label(),
        }
    }
}

impl Label for SequentialNodeKind {
    fn label(&self) -> Cow<str> {
        match self {
            SequentialNodeKind::Join1(..) => "join_1".into(),
            SequentialNodeKind::JoinN(..) => "join_n".into(),
            SequentialNodeKind::Const(_) => "const".into(),
            SequentialNodeKind::Name(name) => format!("name {}", name).into(),
            SequentialNodeKind::OpenScope => "open_scope".into(),
            SequentialNodeKind::CloseScope => "close_scope".into(),
            SequentialNodeKind::Parameter(name, _, _) => format!("parameter {}", name).into(),
            SequentialNodeKind::Declare(name, _, _) => format!("declare_variable {}", name).into(),
            SequentialNodeKind::Assign(..) => "assign".into(),
            SequentialNodeKind::BinOp(..) => "bin_op".into(),
            SequentialNodeKind::Boolean(_) => "boolean".into(),
            SequentialNodeKind::Integer(_) => "integer".into(),
            SequentialNodeKind::Float(_) => "float".into(),
            SequentialNodeKind::AsciiChar(_) => "ascii_char".into(),
            SequentialNodeKind::Utf8Char(_) => "utf-8_char".into(),
            SequentialNodeKind::String(_) => "string".into(),
            SequentialNodeKind::NullTerminatedString(..) => "c-string".into(),
            SequentialNodeKind::Null => "null".into(),
            SequentialNodeKind::Void => "void".into(),
            SequentialNodeKind::Call(..) => "call".into(),
            SequentialNodeKind::DeclareAssign(name, _) => format!("declare_assign {}", name).into(),
            SequentialNodeKind::Member(..) => "member".into(),
            SequentialNodeKind::ArrayAccess(..) => "array_access".into(),
            SequentialNodeKind::StructLiteral(..) => "struct_literal".into(),
            SequentialNodeKind::UnaryOperation(..) => "unary_op".into(),
            SequentialNodeKind::StaticMemberValue(..) => "static_member_value".into(),
            SequentialNodeKind::StaticMemberCall(..) => "static_member_call".into(),
            SequentialNodeKind::SizeOf(_) => "sizeof".into(),
            SequentialNodeKind::SizeOfValue(..) => "sizeof_value".into(),
            SequentialNodeKind::InterpreterSyscall(..) => "interpreter_syscall".into(),
            SequentialNodeKind::IntegerPromote(..) => "integer_promote".into(),
            SequentialNodeKind::StaticAssert(..) => "static_assert".into(),
            SequentialNodeKind::ConformToBool(..) => "conform_to_bool".into(),
            SequentialNodeKind::Is(_, variant) => format!("is {}", variant).into(),
        }
    }
}

impl Label for BranchNode {
    fn label(&self) -> Cow<str> {
        "branch".into()
    }
}

impl Label for TerminatingNode {
    fn label(&self) -> Cow<str> {
        match self {
            TerminatingNode::Return(_) => "return".into(),
            TerminatingNode::Computed(_) => "computed".into(),
            TerminatingNode::Break => "break".into(),
            TerminatingNode::Continue => "continue".into(),
        }
    }
}
