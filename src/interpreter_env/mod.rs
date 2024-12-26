use crate::{
    ast::{
        AstFile, Call, Enum, EnumMember, ExprKind, Field, FieldInitializer, FillBehavior, Function,
        FunctionHead, InterpreterSyscall, Language, Parameter, Parameters, Privacy, StmtKind,
        StructLiteral, Structure, TypeKind,
    },
    interpreter::{
        syscall_handler::{BuildSystemSyscallHandler, ProjectKind},
        Interpreter, InterpreterError,
    },
    ir::{self, InterpreterSyscallKind},
    name::Name,
    resolve::PolyRecipe,
    resolved,
    source_files::Source,
    tag::Tag,
};
use indexmap::IndexMap;

fn thin_void_function(name: impl Into<String>, syscall_kind: InterpreterSyscallKind) -> Function {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());

    let head = FunctionHead {
        name: name.into(),
        givens: vec![],
        parameters: Parameters::default(),
        return_type: void.clone(),
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
        privacy: Privacy::Public,
    };

    Function {
        head,
        stmts: vec![StmtKind::Expr(
            ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                kind: syscall_kind,
                args: vec![],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
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

    let head = FunctionHead {
        name: name.into(),
        givens: vec![],
        parameters: Parameters::normal([Parameter::new(param_name.clone(), ptr_char.clone())]),
        return_type: void.clone(),
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
        privacy: Privacy::Public,
    };

    Function {
        head,
        stmts: vec![StmtKind::Expr(
            ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                kind: syscall_kind,
                args: vec![(
                    ptr_char.clone(),
                    ExprKind::Variable(Name::plain(param_name)).at(source),
                )],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
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
        head: FunctionHead {
            name: "<interpreter entry point>".into(),
            givens: vec![],
            parameters: Parameters::default(),
            return_type: void.clone(),
            is_foreign: false,
            source,
            abide_abi: false,
            tag: Some(Tag::InterpreterEntryPoint),
            privacy: Privacy::Public,
        },
        stmts: vec![StmtKind::Return(Some(call)).at(Source::internal())],
    });

    file.enums.push(Enum {
        name: "ProjectKind".into(),
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
        privacy: Privacy::Private,
    });

    file.structures.push(Structure {
        name: "Project".into(),
        fields: IndexMap::from_iter([(
            "kind".into(),
            Field {
                ast_type: TypeKind::Named(Name::plain("ProjectKind"), vec![]).at(source),
                privacy: Privacy::Public,
                source,
            },
        )]),
        parameters: IndexMap::default(),
        is_packed: false,
        source,
        privacy: Privacy::Private,
    });

    file.structures.push(Structure {
        name: "Dependency".into(),
        fields: IndexMap::from_iter([(
            "name".into(),
            Field {
                ast_type: ptr_char.clone(),
                privacy: Privacy::Private,
                source,
            },
        )]),
        parameters: IndexMap::default(),
        is_packed: false,
        source,
        privacy: Privacy::Private,
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
        head: FunctionHead {
            name: "project".into(),
            givens: vec![],
            parameters: Parameters::normal([
                Parameter::new("name".into(), ptr_char.clone()),
                Parameter::new(
                    "project".into(),
                    TypeKind::Named(Name::plain("Project"), vec![]).at(source),
                ),
            ]),
            return_type: void.clone(),
            abide_abi: false,
            is_foreign: false,
            source,
            tag: None,
            privacy: Privacy::Public,
        },
        stmts: vec![StmtKind::Expr(
            ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                kind: InterpreterSyscallKind::BuildAddProject,
                args: vec![
                    (
                        ptr_char.clone(),
                        ExprKind::Variable(Name::plain("name")).at(source),
                    ),
                    (
                        TypeKind::Named(Name::plain("ProjectKind"), vec![]).at(source),
                        ExprKind::Member(
                            Box::new(ExprKind::Variable(Name::plain("project")).at(source)),
                            "kind".into(),
                            Privacy::Public,
                        )
                        .at(source),
                    ),
                ],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
    });

    file.functions.push(Function {
        head: FunctionHead {
            name: "use".into(),
            givens: vec![],
            parameters: Parameters::normal([
                Parameter::new("as_namespace".into(), ptr_char.clone()),
                Parameter::new(
                    "dependency".into(),
                    TypeKind::Named(Name::plain("Dependency"), vec![]).at(source),
                ),
            ]),
            return_type: void.clone(),
            abide_abi: false,
            is_foreign: false,
            source,
            tag: None,
            privacy: Privacy::Public,
        },
        stmts: vec![StmtKind::Expr(
            ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                kind: InterpreterSyscallKind::UseDependency,
                args: vec![
                    (
                        ptr_char.clone(),
                        ExprKind::Variable(Name::plain("as_namespace")).at(source),
                    ),
                    (
                        TypeKind::Named(Name::plain("Dependency"), vec![]).at(source),
                        ExprKind::Member(
                            Box::new(ExprKind::Variable(Name::plain("dependency")).at(source)),
                            "name".into(),
                            Privacy::Private,
                        )
                        .at(source),
                    ),
                ],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
    });

    file.functions.push(Function {
        head: FunctionHead {
            name: "import".into(),
            givens: vec![],
            parameters: Parameters::normal([Parameter::new("namespace".into(), ptr_char.clone())]),
            return_type: TypeKind::Named(Name::plain("Dependency"), vec![]).at(source),
            abide_abi: false,
            is_foreign: false,
            source,
            tag: None,
            privacy: Privacy::Public,
        },
        stmts: vec![StmtKind::Return(Some(
            ExprKind::StructLiteral(Box::new(StructLiteral {
                ast_type: TypeKind::Named(Name::plain("Dependency"), vec![]).at(source),
                fields: vec![FieldInitializer {
                    name: None,
                    value: ExprKind::Variable(Name::plain("namespace")).at(source),
                }],
                fill_behavior: FillBehavior::Forbid,
                language: Language::Adept,
            }))
            .at(source),
        ))
        .at(source)],
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

    let interpreter_entry_point =
        ir_module
            .functions
            .translate(interpreter_entry_point, PolyRecipe::default(), || {
                Err(InterpreterError::PolymorphicEntryPoint)
            })?;

    let max_steps = Some(1_000_000);
    let handler = BuildSystemSyscallHandler::default();
    let mut interpreter = Interpreter::new(handler, ir_module, max_steps);

    let result = interpreter.run(interpreter_entry_point)?;
    assert!(result.is_literal() && result.unwrap_literal().is_void());
    Ok(interpreter)
}
