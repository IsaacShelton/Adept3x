use crate::{BasicBlockId, InstrRef, conform::UnaryImplicitCast, repr::UnaliasedType};
use ast::{ConformBehavior, FillBehavior, Integer, Language, SizeOfMode, UnaryOperator};
use attributes::Privacy;
use source_files::Source;
use std::{ffi::CStr, fmt::Display};

#[derive(Clone, Debug)]
pub struct EndInstr<'env> {
    pub kind: EndInstrKind<'env>,
    pub source: Source,
}

impl<'env> Display for EndInstr<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl<'env> Display for EndInstrKind<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndInstrKind::IncompleteGoto(name) => writeln!(f, "incomplete goto {}", name),
            EndInstrKind::IncompleteBreak => writeln!(f, "incomplete break"),
            EndInstrKind::IncompleteContinue => writeln!(f, "incomplete continue"),
            EndInstrKind::Return(instr_ref, cast) => {
                if let Some(instr_ref) = instr_ref {
                    write!(f, "return {}", instr_ref)?;
                } else {
                    write!(f, "return")?;
                }

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
                writeln!(f, "")
            }
            EndInstrKind::Jump(bb_id, pre_jump_conform) => {
                if let Some(pre_jump_conform) = pre_jump_conform {
                    writeln!(f, "jump {} as {}", bb_id, pre_jump_conform)
                } else {
                    writeln!(f, "jump {}", bb_id)
                }
            }
            EndInstrKind::Branch(instr_ref, a, b, break_continue) => {
                writeln!(f, "branch {} {} {} {:?}", instr_ref, a, b, break_continue)
            }
            EndInstrKind::NewScope(a, b) => writeln!(f, "new_scope {} {}", a, b),
            EndInstrKind::Unreachable => writeln!(f, "unreachable"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BreakContinue {
    /// Whether the loop is a "positive" loop such as `while` or `for`,
    /// (as opposed to a negative loop like "until" which instead has flipped when_true/when_false)
    #[allow(unused)]
    is_positive: bool,
}

impl BreakContinue {
    pub fn positive() -> Self {
        Self { is_positive: true }
    }

    pub fn negative() -> Self {
        Self { is_positive: false }
    }
}

#[derive(Clone, Debug)]
pub enum EndInstrKind<'env> {
    IncompleteGoto(&'env str),
    IncompleteBreak,
    IncompleteContinue,
    Return(Option<InstrRef>, Option<UnaryImplicitCast<'env>>),
    Jump(BasicBlockId, Option<UnaliasedType<'env>>),
    Branch(InstrRef, BasicBlockId, BasicBlockId, Option<BreakContinue>),
    NewScope(BasicBlockId, BasicBlockId),
    Unreachable,
}

impl<'env> EndInstrKind<'env> {
    pub fn at(self, source: Source) -> EndInstr<'env> {
        EndInstr { kind: self, source }
    }
}

#[derive(Clone, Debug)]
pub struct Instr<'env> {
    pub kind: InstrKind<'env>,
    pub typed: Option<UnaliasedType<'env>>,
    pub source: Source,
}

impl<'env> Display for Instr<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(typed) = self.typed {
            writeln!(f, "{}  ->  {}", self.kind, typed)?;
        } else {
            writeln!(f, "{}", self.kind)?;
        }

        Ok(())
    }
}

impl<'env> Display for InstrKind<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstrKind::Phi(items, conform_behavior) => {
                write!(f, "phi ")?;

                for (bb_id, instr_ref) in items.iter() {
                    if let Some(instr_ref) = instr_ref {
                        write!(f, "[{} {}] ", bb_id, instr_ref)?;
                    } else {
                        write!(f, "[{} None] ", bb_id)?;
                    }
                }

                write!(f, "{:?}", conform_behavior)?;
            }
            InstrKind::Name(name) => write!(f, "name {}", name)?,
            InstrKind::Parameter(name, ty, index) => {
                write!(f, "param {} {} {}", name, ty, index)?;
            }
            InstrKind::Declare(name, ty, instr_ref, cast) => {
                if let Some(instr_ref) = instr_ref {
                    write!(f, "declare {} {} {}", name, ty, instr_ref)?;
                } else {
                    write!(f, "declare {} {}", name, ty)?;
                }

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
            }
            InstrKind::Assign(dest, src) => {
                write!(f, "assign {} {}", dest, src)?;
            }
            InstrKind::BinOp(a, op, b, language) => {
                write!(f, "bin_op {} {} {} {:?}", a, op, b, language)?;
            }
            InstrKind::BooleanLiteral(value) => {
                write!(f, "bool_lit {}", value)?;
            }
            InstrKind::IntegerLiteral(value) => {
                write!(f, "integer_lit {:?}", value)?;
            }
            InstrKind::FloatLiteral(value) => {
                write!(f, "float_lit {}", value)?;
            }
            InstrKind::AsciiCharLiteral(value) => {
                write!(f, "ascii_char_lit {}", value)?;
            }
            InstrKind::Utf8CharLiteral(value) => {
                write!(f, "utf8_char_lit {}", value)?;
            }
            InstrKind::StringLiteral(value) => {
                write!(f, "string_lit {:?}", value)?;
            }
            InstrKind::NullTerminatedStringLiteral(value) => {
                write!(f, "null_terminated_string_lit {:?}", value)?;
            }
            InstrKind::NullLiteral => write!(f, "null_lit")?,
            InstrKind::VoidLiteral => write!(f, "void_lit")?,
            InstrKind::Call(call) => {
                write!(f, "call {} (", call.name)?;

                for (i, instr_ref) in call.args.iter().enumerate() {
                    if i + 1 == call.args.len() {
                        write!(f, "{}", instr_ref)?;
                    } else {
                        write!(f, "{}, ", instr_ref)?;
                    }
                }

                write!(f, ") {:?} {:?}", call.generics, call.expected_to_return)?;
            }
            InstrKind::DeclareAssign(name, instr_ref, cast) => {
                write!(f, "declare_assign {} {}", name, instr_ref)?;

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
            }
            InstrKind::Member(subject, name, privacy) => {
                write!(f, "member {} {} {}", subject, name, privacy)?;
            }
            InstrKind::ArrayAccess(subject, index) => {
                write!(f, "array_access {} {}", subject, index)?;
            }
            InstrKind::StructLiteral(struct_lit) => {
                write!(f, "struct_lit {} {{ ", struct_lit.ast_type)?;

                for (i, field) in struct_lit.fields.iter().enumerate() {
                    if i + 1 == struct_lit.fields.len() {
                        write!(
                            f,
                            "{}: {}, ",
                            field.name.unwrap_or("<unnamed>"),
                            field.value
                        )?;
                    } else {
                        write!(f, "{}: {} ", field.name.unwrap_or("<unnamed>"), field.value)?;
                    }
                }

                write!(
                    f,
                    "}} {:?} {:?}",
                    struct_lit.fill_behavior, struct_lit.language
                )?;
            }
            InstrKind::UnaryOperation(op, instr_ref) => {
                write!(f, "unary_op {:?} {}", op, instr_ref)?;
            }
            InstrKind::SizeOf(ty, mode) => {
                write!(f, "sizeof {} {:?}", ty, mode)?;
            }
            InstrKind::SizeOfValue(instr_ref, mode) => {
                write!(f, "sizeof_value {} {:?}", instr_ref, mode)?;
            }
            InstrKind::InterpreterSyscall(syscall) => {
                write!(f, "interp_systcall {:?}", syscall)?;
            }
            InstrKind::IntegerPromote(instr_ref) => {
                write!(f, "integer_promote {}", instr_ref)?;
            }
            InstrKind::ConformToBool(instr_ref, language, cast) => {
                write!(f, "conform_to_bool {} {:?}", instr_ref, language)?;

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
            }
            InstrKind::Is(instr_ref, variant) => {
                write!(f, "is {} {}", instr_ref, variant)?;
            }
            InstrKind::LabelLiteral(label) => {
                write!(f, "label_lit {}", label)?;
            }
        }
        Ok(())
    }
}

// Getting this down to 32 is going to take some extreme manual layout optimization
const _: () = assert!(std::mem::size_of::<InstrKind>() <= 48);
const _: () = assert!(std::mem::align_of::<InstrKind>() <= 8);

#[derive(Clone, Debug)]
pub enum InstrKind<'env> {
    Phi(
        &'env [(BasicBlockId, Option<InstrRef>)],
        Option<ConformBehavior>,
    ),
    Name(&'env str),
    Parameter(&'env str, &'env ast::Type, u32),
    Declare(
        &'env str,
        &'env ast::Type,
        Option<InstrRef>,
        Option<UnaryImplicitCast<'env>>,
    ),
    Assign(InstrRef, InstrRef),
    BinOp(InstrRef, ast::BasicBinaryOperator, InstrRef, Language),
    BooleanLiteral(bool),
    IntegerLiteral(&'env Integer),
    FloatLiteral(f64),
    AsciiCharLiteral(u8),
    Utf8CharLiteral(&'env str),
    StringLiteral(&'env str),
    NullTerminatedStringLiteral(&'env CStr),
    NullLiteral,
    VoidLiteral,
    Call(&'env CallInstr<'env>),
    DeclareAssign(&'env str, InstrRef, Option<UnaryImplicitCast<'env>>),
    Member(InstrRef, &'env str, Privacy),
    ArrayAccess(InstrRef, InstrRef),
    StructLiteral(&'env StructLiteralInstr<'env>),
    UnaryOperation(UnaryOperator, InstrRef),
    SizeOf(&'env ast::Type, Option<SizeOfMode>),
    SizeOfValue(InstrRef, Option<SizeOfMode>),
    InterpreterSyscall(&'env InterpreterSyscallInstr<'env>),
    IntegerPromote(InstrRef),
    ConformToBool(InstrRef, Language, Option<UnaryImplicitCast<'env>>),
    Is(InstrRef, &'env str),
    LabelLiteral(&'env str),
}

impl<'env> InstrKind<'env> {
    pub fn at(self, source: Source) -> Instr<'env> {
        Instr {
            kind: self,
            source,
            typed: None,
        }
    }
}

#[derive(Debug)]
pub struct CallInstr<'env> {
    pub name: &'env str,
    pub args: &'env [InstrRef],
    pub expected_to_return: Option<&'env ast::Type>,
    pub generics: &'env [&'env ast::Type],
}

#[derive(Debug)]
pub struct StructLiteralInstr<'env> {
    pub ast_type: &'env ast::Type,
    pub fields: &'env [FieldInitializer<'env>],
    pub fill_behavior: FillBehavior,
    pub language: Language,
}

#[derive(Debug)]
pub struct FieldInitializer<'env> {
    pub name: Option<&'env str>,
    pub value: InstrRef,
}

#[derive(Debug)]
pub struct InterpreterSyscallInstr<'env> {
    pub kind: interpreter_api::Syscall,
    pub args: &'env [(&'env ast::Type, InstrRef)],
    pub result_type: &'env ast::Type,
}
