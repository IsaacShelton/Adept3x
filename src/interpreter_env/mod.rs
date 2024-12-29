use crate::{
    asg::Asg,
    ast::{
        AstFile, Call, Enum, EnumMember, ExprKind, Field, FieldInitializer, FillBehavior, Func,
        FuncHead, InterpreterSyscall, Language, Param, Params, Privacy, StmtKind, Struct,
        StructLiteral, TypeKind,
    },
    interpreter::{
        syscall_handler::{BuildSystemSyscallHandler, ProjectKind},
        Interpreter, InterpreterError,
    },
    ir::{self, InterpreterSyscallKind},
    name::Name,
    resolve::PolyRecipe,
    source_files::Source,
    tag::Tag,
};
use indexmap::IndexMap;

fn thin_void_func(name: impl Into<String>, syscall_kind: InterpreterSyscallKind) -> Func {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());

    let head = FuncHead {
        name: name.into(),
        givens: vec![],
        params: Params::default(),
        return_type: void.clone(),
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
        privacy: Privacy::Public,
    };

    Func {
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

fn thin_cstring_func(
    name: impl Into<String>,
    param_name: impl Into<String>,
    syscall_kind: InterpreterSyscallKind,
) -> Func {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());
    let ptr_char = TypeKind::Ptr(Box::new(TypeKind::char().at(source))).at(source);
    let param_name = param_name.into();

    let head = FuncHead {
        name: name.into(),
        givens: vec![],
        params: Params::normal([Param::new(param_name.clone(), ptr_char.clone())]),
        return_type: void.clone(),
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
        privacy: Privacy::Public,
    };

    Func {
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
    let ptr_char = TypeKind::Ptr(Box::new(TypeKind::char().at(source))).at(source);

    // Call to function we actually care about
    let call = ExprKind::Call(Box::new(Call {
        name: Name::plain("main"),
        arguments: vec![],
        expected_to_return: Some(void.clone()),
        generics: vec![],
    }))
    .at(Source::internal());

    file.funcs.push(Func {
        head: FuncHead {
            name: "<interpreter entry point>".into(),
            givens: vec![],
            params: Params::default(),
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

    file.structs.push(Struct {
        name: "Project".into(),
        fields: IndexMap::from_iter([(
            "kind".into(),
            Field {
                ast_type: TypeKind::Named(Name::plain("ProjectKind"), vec![]).at(source),
                privacy: Privacy::Public,
                source,
            },
        )]),
        params: IndexMap::default(),
        is_packed: false,
        source,
        privacy: Privacy::Private,
    });

    file.structs.push(Struct {
        name: "Dependency".into(),
        fields: IndexMap::from_iter([(
            "name".into(),
            Field {
                ast_type: ptr_char.clone(),
                privacy: Privacy::Private,
                source,
            },
        )]),
        params: IndexMap::default(),
        is_packed: false,
        source,
        privacy: Privacy::Private,
    });

    file.funcs.push(thin_cstring_func(
        "println",
        "message",
        InterpreterSyscallKind::Println,
    ));

    file.funcs.push(thin_cstring_func(
        "adept",
        "version_string",
        InterpreterSyscallKind::BuildSetAdeptVersion,
    ));

    file.funcs.push(thin_cstring_func(
        "link",
        "filename",
        InterpreterSyscallKind::BuildLinkFilename,
    ));

    file.funcs.push(thin_cstring_func(
        "linkFramework",
        "framework_name",
        InterpreterSyscallKind::BuildLinkFrameworkName,
    ));

    file.funcs.push(thin_cstring_func(
        "experimental",
        "experiment",
        InterpreterSyscallKind::Experimental,
    ));

    file.funcs.push(thin_cstring_func(
        "importNamespace",
        "namespace",
        InterpreterSyscallKind::ImportNamespace,
    ));

    file.funcs.push(thin_void_func(
        "dontAssumeIntAtLeast32Bits",
        InterpreterSyscallKind::DontAssumeIntAtLeast32Bits,
    ));

    file.funcs.push(Func {
        head: FuncHead {
            name: "project".into(),
            givens: vec![],
            params: Params::normal([
                Param::new("name".into(), ptr_char.clone()),
                Param::new(
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

    file.funcs.push(Func {
        head: FuncHead {
            name: "use".into(),
            givens: vec![],
            params: Params::normal([
                Param::new("as_namespace".into(), ptr_char.clone()),
                Param::new(
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

    file.funcs.push(Func {
        head: FuncHead {
            name: "import".into(),
            givens: vec![],
            params: Params::normal([Param::new("namespace".into(), ptr_char.clone())]),
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
    asg: &'a Asg<'_>,
    ir_module: &'a ir::Module,
) -> Result<Interpreter<'a, BuildSystemSyscallHandler>, InterpreterError> {
    let (interpreter_entry_point, _fn) = asg
        .funcs
        .iter()
        .find(|(_, f)| f.tag == Some(Tag::InterpreterEntryPoint))
        .unwrap();

    let interpreter_entry_point =
        ir_module
            .funcs
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
