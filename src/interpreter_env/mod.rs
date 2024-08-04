use crate::{
    ast::{self, AstFile, IntegerBits},
    interpreter::{
        syscall_handler::{BuildSystemSyscallHandler, ProjectKind},
        Interpreter, InterpreterError,
    },
    ir::{self, IntegerSign, InterpreterSyscallKind},
    resolved,
    source_files::Source,
    tag::Tag,
};
use indexmap::IndexMap;

fn thin_cstring_function(
    name: impl ToString,
    param_name: impl ToString,
    syscall_kind: InterpreterSyscallKind,
) -> ast::Function {
    let source = Source::internal();
    let void = ast::TypeKind::Void.at(Source::internal());
    let ptr_u8 = ast::TypeKind::Pointer(Box::new(
        ast::TypeKind::Integer {
            bits: IntegerBits::Bits8,
            sign: IntegerSign::Unsigned,
        }
        .at(source),
    ))
    .at(source);

    ast::Function {
        name: name.to_string(),
        parameters: ast::Parameters {
            required: vec![ast::Parameter::new(param_name.to_string(), ptr_u8.clone())],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![ast::StmtKind::Expr(
            ast::ExprKind::InterpreterSyscall(Box::new(ast::InterpreterSyscall {
                kind: syscall_kind,
                args: vec![(
                    ptr_u8.clone(),
                    ast::ExprKind::Variable(param_name.to_string()).at(source),
                )],
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
    let void = ast::TypeKind::Void.at(Source::internal());
    let ptr_u8 = ast::TypeKind::Pointer(Box::new(
        ast::TypeKind::Integer {
            bits: IntegerBits::Bits8,
            sign: IntegerSign::Unsigned,
        }
        .at(source),
    ))
    .at(source);

    // Call to function we actually care about
    let call = ast::ExprKind::Call(Box::new(ast::Call {
        function_name: "main".into(),
        arguments: vec![],
        expected_to_return: Some(void.clone()),
    }))
    .at(Source::internal());

    file.functions.push(ast::Function {
        name: "<interpreter entry point>".into(),
        parameters: ast::Parameters::default(),
        return_type: void.clone(),
        stmts: vec![ast::StmtKind::Return(Some(call)).at(Source::internal())],
        is_foreign: false,
        source,
        abide_abi: false,
        tag: Some(Tag::InterpreterEntryPoint),
    });

    file.enums.insert(
        "ProjectKind".into(),
        ast::Enum {
            backing_type: Some(
                ast::TypeKind::Integer {
                    bits: IntegerBits::Bits64,
                    sign: IntegerSign::Unsigned,
                }
                .at(source),
            ),
            source,
            members: IndexMap::from_iter([
                (
                    "ConsoleApp".into(),
                    ast::EnumMember {
                        value: (ProjectKind::ConsoleApp as u64).into(),
                        explicit_value: true,
                    },
                ),
                (
                    "WindowedApp".into(),
                    ast::EnumMember {
                        value: (ProjectKind::WindowedApp as u64).into(),
                        explicit_value: true,
                    },
                ),
            ]),
        },
    );

    file.structures.push(ast::Structure {
        name: "Project".into(),
        fields: IndexMap::from_iter([(
            "kind".into(),
            ast::Field {
                ast_type: ast::TypeKind::Named("ProjectKind".into()).at(source),
                privacy: ast::Privacy::Public,
                source,
            },
        )]),
        is_packed: false,
        prefer_pod: false,
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

    file.functions.push(ast::Function {
        name: "project".into(),
        parameters: ast::Parameters {
            required: vec![
                ast::Parameter::new("name".into(), ptr_u8.clone()),
                ast::Parameter::new(
                    "project".into(),
                    ast::TypeKind::Named("Project".into()).at(source),
                ),
            ],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![ast::StmtKind::Expr(
            ast::ExprKind::InterpreterSyscall(Box::new(ast::InterpreterSyscall {
                kind: InterpreterSyscallKind::BuildAddProject,
                args: vec![
                    (
                        ptr_u8.clone(),
                        ast::ExprKind::Variable("name".into()).at(source),
                    ),
                    (
                        ast::TypeKind::Named("ProjectKind".into()).at(source),
                        ast::ExprKind::Member(
                            Box::new(ast::ExprKind::Variable("project".into()).at(source)),
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
