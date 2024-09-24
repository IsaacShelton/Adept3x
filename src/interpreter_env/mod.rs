use crate::{
    ast::{
        AstFile, Call, Enum, EnumMember, ExprKind, Field, Function, InterpreterSyscall, Parameter,
        Parameters, Privacy, StmtKind, Structure, TypeKind,
    },
    interpreter::{
        syscall_handler::{BuildSystemSyscallHandler, ProjectKind},
        Interpreter, InterpreterError,
    },
    ir::{self, InterpreterSyscallKind},
    name::Name,
    resolved,
    source_files::Source,
    tag::Tag,
};
use indexmap::IndexMap;

fn thin_void_function(name: impl Into<String>, syscall_kind: InterpreterSyscallKind) -> Function {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());

    Function {
        name: Name::plain(name),
        parameters: Parameters {
            required: vec![],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![StmtKind::Expr(
            ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                kind: syscall_kind,
                args: vec![],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
    }
}

fn thin_cstring_function(
    name: impl Into<String>,
    param_name: impl Into<String>,
    syscall_kind: InterpreterSyscallKind,
) -> Function {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());
    let ptr_char = TypeKind::Pointer(Box::new(TypeKind::char().at(source))).at(source);
    let param_name = param_name.into();
    Function {
        name: Name::plain(name.into()),
        parameters: Parameters {
            required: vec![Parameter::new(param_name.clone(), ptr_char.clone())],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![StmtKind::Expr(
            ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                kind: syscall_kind,
                args: vec![(ptr_char.clone(), ExprKind::Variable(param_name).at(source))],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
    }
}

pub fn setup_build_system_interpreter_symbols(file: &mut AstFile) {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());
    let ptr_char = TypeKind::Pointer(Box::new(TypeKind::char().at(source))).at(source);

    // Call to function we actually care about
    let call = ExprKind::Call(Box::new(Call {
        function_name: Name::plain("main"),
        arguments: vec![],
        expected_to_return: Some(void.clone()),
        generics: vec![],
    }))
    .at(Source::internal());

    file.functions.push(Function {
        name: Name::plain("<interpreter entry point>"),
        parameters: Parameters::default(),
        return_type: void.clone(),
        stmts: vec![StmtKind::Return(Some(call)).at(Source::internal())],
        is_foreign: false,
        source,
        abide_abi: false,
        tag: Some(Tag::InterpreterEntryPoint),
    });

    file.enums.insert(
        Name::plain("ProjectKind"),
        Enum {
            backing_type: Some(TypeKind::u64().at(source)),
            source,
            members: IndexMap::from_iter([
                (
                    "ConsoleApp".into(),
                    EnumMember {
                        value: (ProjectKind::ConsoleApp as u64).into(),
                        explicit_value: true,
                    },
                ),
                (
                    "WindowedApp".into(),
                    EnumMember {
                        value: (ProjectKind::WindowedApp as u64).into(),
                        explicit_value: true,
                    },
                ),
            ]),
        },
    );

    file.structures.push(Structure {
        name: Name::plain("Project"),
        fields: IndexMap::from_iter([(
            "kind".into(),
            Field {
                ast_type: TypeKind::Named(Name::plain("ProjectKind")).at(source),
                privacy: Privacy::Public,
                source,
            },
        )]),
        is_packed: false,
        source,
    });

    file.functions.push(thin_cstring_function(
        "println",
        "message",
        InterpreterSyscallKind::Println,
    ));

    file.functions.push(thin_cstring_function(
        "adept",
        "version_string",
        InterpreterSyscallKind::BuildSetAdeptVersion,
    ));

    file.functions.push(thin_cstring_function(
        "link",
        "filename",
        InterpreterSyscallKind::BuildLinkFilename,
    ));

    file.functions.push(thin_cstring_function(
        "linkFramework",
        "framework_name",
        InterpreterSyscallKind::BuildLinkFrameworkName,
    ));

    file.functions.push(thin_cstring_function(
        "experimental",
        "experiment",
        InterpreterSyscallKind::Experimental,
    ));

    file.functions.push(thin_cstring_function(
        "importNamespace",
        "namespace",
        InterpreterSyscallKind::ImportNamespace,
    ));

    file.functions.push(thin_void_function(
        "dontAssumeIntAtLeast32Bits",
        InterpreterSyscallKind::DontAssumeIntAtLeast32Bits,
    ));

    file.functions.push(Function {
        name: Name::plain("project"),
        parameters: Parameters {
            required: vec![
                Parameter::new("name".into(), ptr_char.clone()),
                Parameter::new(
                    "project".into(),
                    TypeKind::Named(Name::plain("Project")).at(source),
                ),
            ],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![StmtKind::Expr(
            ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                kind: InterpreterSyscallKind::BuildAddProject,
                args: vec![
                    (
                        ptr_char.clone(),
                        ExprKind::Variable("name".into()).at(source),
                    ),
                    (
                        TypeKind::Named(Name::plain("ProjectKind")).at(source),
                        ExprKind::Member(
                            Box::new(ExprKind::Variable("project".into()).at(source)),
                            "kind".into(),
                        )
                        .at(source),
                    ),
                ],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
    });
}

pub fn run_build_system_interpreter<'a>(
    resolved_ast: &'a resolved::Ast<'_>,
    ir_module: &'a ir::Module,
) -> Result<Interpreter<'a, BuildSystemSyscallHandler>, InterpreterError> {
    let (interpreter_entry_point, _fn) = resolved_ast
        .functions
        .iter()
        .find(|(_, f)| f.tag == Some(Tag::InterpreterEntryPoint))
        .unwrap();

    let max_steps = Some(1_000_000);
    let handler = BuildSystemSyscallHandler::default();
    let mut interpreter = Interpreter::new(handler, ir_module, max_steps);

    let result = interpreter.run(interpreter_entry_point)?;
    assert!(result.is_literal() && result.unwrap_literal().is_void());
    Ok(interpreter)
}
