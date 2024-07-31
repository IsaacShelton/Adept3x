use indexmap::IndexMap;

use crate::{
    ast::{self, Ast, IntegerBits, Source},
    interpreter::{
        syscall_handler::{BuildSystemSyscallHandler, ProjectKind},
        Interpreter,
    },
    ir::{self, IntegerSign, InterpreterSyscallKind},
    resolved,
    tag::Tag,
};

pub fn setup_build_system_interpreter_symbols(ast: &mut Ast) {
    // We assume only working with single file for now
    assert_eq!(ast.files.len(), 1);

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

    // Create entry point for interpreter which will make the call
    let (_, file) = ast.files.iter_mut().next().unwrap();
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

    file.functions.push(ast::Function {
        name: "println".into(),
        parameters: ast::Parameters {
            required: vec![ast::Parameter::new("message".into(), ptr_u8.clone())],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![ast::StmtKind::Expr(
            ast::ExprKind::InterpreterSyscall(Box::new(ast::InterpreterSyscall {
                kind: InterpreterSyscallKind::Println,
                args: vec![(
                    ptr_u8.clone(),
                    ast::ExprKind::Variable("message".into()).at(source),
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
    });

    file.functions.push(ast::Function {
        name: "adept".into(),
        parameters: ast::Parameters {
            required: vec![ast::Parameter::new("version_string".into(), ptr_u8.clone())],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![ast::StmtKind::Expr(
            ast::ExprKind::InterpreterSyscall(Box::new(ast::InterpreterSyscall {
                kind: InterpreterSyscallKind::BuildSetAdeptVersion,
                args: vec![(
                    ptr_u8.clone(),
                    ast::ExprKind::Variable("version_string".into()).at(source),
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
    });

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
) -> Result<Interpreter<'a, BuildSystemSyscallHandler>, ()> {
    let (interpreter_entry_point, _fn) = resolved_ast
        .functions
        .iter()
        .find(|(_, f)| f.tag == Some(Tag::InterpreterEntryPoint))
        .unwrap();

    let max_steps = Some(1_000_000);
    let handler = BuildSystemSyscallHandler::default();
    let mut interpreter = Interpreter::new(handler, ir_module, max_steps);

    match interpreter.run(interpreter_entry_point) {
        Ok(result) => assert!(result.is_literal() && result.unwrap_literal().is_void()),
        Err(err) => {
            eprintln!("build script error: {}", err);
            return Err(());
        }
    }

    Ok(interpreter)
}
