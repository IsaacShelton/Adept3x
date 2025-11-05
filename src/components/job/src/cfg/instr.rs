use crate::{
    BasicBlockId, CfgValue,
    conform::UnaryCast,
    ir::BinOp,
    module_graph::ModuleView,
    repr::{FuncHead, TypeDisplayerDisambiguation, UnaliasedType, VariableRef},
};
use ast::{ConformBehavior, FillBehavior, Integer, Language, NamePath, SizeOfMode, UnaryOperator};
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
            EndInstrKind::Return(value, cast) => {
                write!(f, "return {}", value)?;

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
                writeln!(f, "")
            }
            EndInstrKind::Jump(bb_id, _value, typed_unary_cast, _to_ty) => {
                if let Some(typed_unary_cast) = typed_unary_cast {
                    writeln!(f, "jump {} as {:?}", bb_id, typed_unary_cast)
                } else {
                    writeln!(f, "jump {}", bb_id)
                }
            }
            EndInstrKind::Branch(instr_ref, a, b, break_continue) => {
                writeln!(f, "branch {} {} {} {:?}", instr_ref, a, b, break_continue)
            }
            EndInstrKind::NewScope(a, b) => writeln!(f, "new_scope {} {}", a, b),
            EndInstrKind::Unreachable => writeln!(f, "unreachable"),
            EndInstrKind::ExitInterpreter(value, _, _) => {
                writeln!(f, "exit_interpreter {}", value)
            }
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
    Return(CfgValue, Option<UnaryCast<'env>>),
    Jump(
        BasicBlockId,
        CfgValue,
        Option<UnaryCast<'env>>,
        Option<UnaliasedType<'env>>,
    ),
    Branch(CfgValue, BasicBlockId, BasicBlockId, Option<BreakContinue>),
    NewScope(BasicBlockId, BasicBlockId),
    Unreachable,
    ExitInterpreter(
        CfgValue,
        Option<UnaliasedType<'env>>,
        Option<UnaryCast<'env>>,
    ),
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

impl<'env> Instr<'env> {
    pub fn display<'a, 'b>(
        &'a self,
        view: &'b ModuleView<'env>,
        disambiguation: &'a TypeDisplayerDisambiguation<'env>,
    ) -> InstrDisplayer<'a, 'b, 'env> {
        InstrDisplayer {
            instr: self,
            view,
            disambiguation,
        }
    }
}

pub struct InstrDisplayer<'a, 'b, 'env: 'a + 'b> {
    instr: &'a Instr<'env>,
    view: &'b ModuleView<'env>,
    disambiguation: &'a TypeDisplayerDisambiguation<'env>,
}

impl<'a, 'b, 'env: 'a + 'b> Display for InstrDisplayer<'a, 'b, 'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(typed) = &self.instr.typed {
            writeln!(
                f,
                "{}  ->  {}",
                self.instr.kind,
                typed.display(self.view, self.disambiguation)
            )?;
        } else {
            writeln!(f, "{}", self.instr.kind)?;
        }

        Ok(())
    }
}

impl<'env> Display for InstrKind<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstrKind::Phi {
                possible_incoming,
                conform_behavior,
            } => {
                write!(f, "phi ")?;

                for (bb_id, cfg_value) in possible_incoming.iter() {
                    write!(f, "[{} {}] ", bb_id, cfg_value)?;
                }

                write!(f, "{:?}", conform_behavior)?;
            }
            InstrKind::Name(name, _) => write!(f, "name {}", name)?,
            InstrKind::DeclareParameter(name, ty, index, _) => {
                write!(f, "param {} {} {}", name, ty, index)?;
            }
            InstrKind::Declare(name, ty, cfg_value, cast, _) => {
                if let Some(cfg_value) = cfg_value {
                    write!(f, "declare {} {} {}", name, ty, cfg_value)?;
                } else {
                    write!(f, "declare {} {}", name, ty)?;
                }

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
            }
            InstrKind::IntoDest(dest, cast) => {
                write!(f, "into_dest {}", dest)?;

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
            }
            InstrKind::Assign {
                dest,
                src,
                src_cast: cast,
            } => {
                write!(f, "assign {} {}", dest, src)?;

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
            }
            InstrKind::BinOp(a, op, b, language, _, _, _) => {
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
            InstrKind::Call(call, call_target) => {
                write!(f, "call {} (", call.name_path)?;

                for (i, cfg_value) in call.args.iter().enumerate() {
                    if i + 1 == call.args.len() {
                        write!(f, "{}", cfg_value)?;
                    } else {
                        write!(f, "{}, ", cfg_value)?;
                    }
                }

                write!(
                    f,
                    ") {:?} {:?} {:?}",
                    call.generics,
                    call.expected_to_return,
                    call_target
                        .as_ref()
                        .map(|call_target| call_target.arg_casts)
                )?;
            }
            InstrKind::DeclareAssign(name, cfg_value, cast, _) => {
                write!(f, "declare_assign {} {}", name, cfg_value)?;

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
            InstrKind::StructLiteral(struct_lit, _casts) => {
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
                    struct_lit.fill_behavior, struct_lit.conform_behavior
                )?;
            }
            InstrKind::UnaryOperation(op, cfg_value, cast) => {
                write!(f, "unary_op {:?} {}", op, cfg_value)?;

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
            }
            InstrKind::SizeOf(ty, mode) => {
                write!(f, "sizeof {} {:?}", ty, mode)?;
            }
            InstrKind::SizeOfValue(cfg_value, mode) => {
                write!(f, "sizeof_value {} {:?}", cfg_value, mode)?;
            }
            InstrKind::InterpreterSyscall(syscall) => {
                write!(f, "interp_systcall {:?}", syscall)?;
            }
            InstrKind::IntegerPromote(cfg_value) => {
                write!(f, "integer_promote {}", cfg_value)?;
            }
            InstrKind::ConformToBool(cfg_value, language, cast) => {
                write!(f, "conform_to_bool {} {:?}", cfg_value, language)?;

                if let Some(cast) = cast {
                    write!(f, "\n        | casts to: {:?}", cast)?;
                }
            }
            InstrKind::Is(cfg_value, variant) => {
                write!(f, "is {} {}", cfg_value, variant)?;
            }
            InstrKind::LabelLiteral(label) => {
                write!(f, "label_lit {}", label)?;
            }
            InstrKind::Comptime(_) => {
                write!(f, "comptime <...>")?;
            }
        }
        Ok(())
    }
}

// Getting this down is going to take some extreme manual layout optimization
const _: () = assert!(std::mem::size_of::<InstrKind>() <= 88);
const _: () = assert!(std::mem::align_of::<InstrKind>() <= 8);

#[derive(Clone, Debug)]
pub enum InstrKind<'env> {
    Phi {
        possible_incoming: &'env [(BasicBlockId, CfgValue)],
        conform_behavior: Option<ConformBehavior>,
    },
    Name(&'env str, Option<VariableRef<'env>>),
    DeclareParameter(&'env str, &'env ast::Type, u32, Option<VariableRef<'env>>),
    Declare(
        &'env str,
        &'env ast::Type,
        Option<CfgValue>,
        Option<UnaryCast<'env>>,
        Option<VariableRef<'env>>,
    ),
    IntoDest(CfgValue, Option<UnaryCast<'env>>),
    Assign {
        dest: CfgValue,
        src: CfgValue,
        src_cast: Option<UnaryCast<'env>>,
    },
    BinOp(
        CfgValue,
        ast::BasicBinaryOperator,
        CfgValue,
        ConformBehavior,
        Option<UnaryCast<'env>>,
        Option<UnaryCast<'env>>,
        Option<(BinOp, UnaliasedType<'env>)>,
    ),
    BooleanLiteral(bool),
    IntegerLiteral(&'env Integer),
    FloatLiteral(f64),
    AsciiCharLiteral(u8),
    Utf8CharLiteral(&'env str),
    StringLiteral(&'env str),
    NullTerminatedStringLiteral(&'env CStr),
    NullLiteral,
    VoidLiteral,
    Call(&'env CallInstr<'env>, Option<CallTarget<'env>>),
    DeclareAssign(
        &'env str,
        CfgValue,
        Option<UnaryCast<'env>>,
        Option<VariableRef<'env>>,
    ),
    Member(CfgValue, &'env str, Privacy),
    ArrayAccess(CfgValue, CfgValue),
    StructLiteral(
        &'env StructLiteralInstr<'env>,
        Option<&'env [(usize, Option<UnaryCast<'env>>)]>,
    ),
    UnaryOperation(UnaryOperator, CfgValue, Option<UnaryCast<'env>>),
    SizeOf(&'env ast::Type, Option<SizeOfMode>),
    SizeOfValue(CfgValue, Option<SizeOfMode>),
    InterpreterSyscall(&'env InterpreterSyscallInstr<'env>),
    IntegerPromote(CfgValue),
    ConformToBool(CfgValue, Language, Option<UnaryCast<'env>>),
    Is(CfgValue, &'env str),
    LabelLiteral(&'env str),
    Comptime(&'env ast::Expr),
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
    pub name_path: &'env NamePath,
    pub args: &'env [CfgValue],
    pub expected_to_return: Option<&'env ast::Type>,
    pub generics: &'env [&'env ast::Type],
}

#[derive(Clone, Debug)]
pub struct CallTarget<'env> {
    pub callee: &'env FuncHead<'env>,
    pub arg_casts: &'env [Option<UnaryCast<'env>>],
    pub variadic_arg_types: &'env [UnaliasedType<'env>],
    pub view: &'env ModuleView<'env>,
}

impl<'env> CallTarget<'env> {
    pub fn get_param_or_arg_type(&self, i: usize) -> UnaliasedType<'env> {
        self.callee
            .params
            .required
            .get(i)
            .map(|param| param.ty)
            .or_else(|| {
                self.variadic_arg_types
                    .get(i - self.callee.params.required.len())
                    .copied()
            })
            .expect("call target info to know parameter or argument type")
    }
}

#[derive(Debug)]
pub struct StructLiteralInstr<'env> {
    pub ast_type: &'env ast::Type,
    pub fields: &'env [FieldInitializer<'env>],
    pub fill_behavior: FillBehavior,
    pub conform_behavior: ConformBehavior,
}

#[derive(Debug)]
pub struct FieldInitializer<'env> {
    pub name: Option<&'env str>,
    pub value: CfgValue,
}

#[derive(Debug)]
pub struct InterpreterSyscallInstr<'env> {
    pub kind: interpreter_api::Syscall,
    pub args: &'env [(&'env ast::Type, CfgValue)],
    pub result_type: &'env ast::Type,
}
