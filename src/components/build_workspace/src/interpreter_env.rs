use ast::{
    Call, Enum, EnumMember, ExprKind, Field, FieldInitializer, FillBehavior, Func, FuncHead,
    InterpreterSyscall, Language, NamePath, Param, Params, RawAstFile, StmtKind, Struct,
    StructLiteral, TypeKind, TypeParams,
};
use attributes::{Exposure, Privacy, SymbolOwnership, Tag};
use indexmap::IndexMap;
use interpret::{
    Interpreter, InterpreterError,
    syscall_handler::{BuildSystemSyscallHandler, ProjectKind},
};
use interpreter_api::Syscall;
use source_files::Source;

fn thin_void_func(name: impl Into<String>, syscall: Syscall) -> Func {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());

    let head = FuncHead {
        name: name.into(),
        type_params: TypeParams::default(),
        givens: vec![],
        params: Params::default(),
        return_type: void.clone(),
        abide_abi: false,
        ownership: SymbolOwnership::Owned(Exposure::Hidden),
        source,
        tag: None,
        privacy: Privacy::Public,
    };

    Func::new(
        head,
        vec![
            StmtKind::Expr(
                ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                    kind: syscall,
                    args: vec![],
                    result_type: void.clone(),
                }))
                .at(source),
            )
            .at(source),
        ],
    )
}

fn thin_cstring_func(
    name: impl Into<String>,
    param_name: impl Into<String>,
    syscall: Syscall,
) -> Func {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());
    let ptr_char = TypeKind::Ptr(Box::new(TypeKind::char().at(source))).at(source);
    let param_name = param_name.into();

    let head = FuncHead {
        name: name.into(),
        type_params: TypeParams::default(),
        givens: vec![],
        params: Params::normal([Param::named(param_name.clone(), ptr_char.clone())]),
        return_type: void.clone(),
        abide_abi: false,
        ownership: SymbolOwnership::default(),
        source,
        tag: None,
        privacy: Privacy::Public,
    };

    Func::new(
        head,
        vec![
            StmtKind::Expr(
                ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                    kind: syscall,
                    args: vec![(
                        ptr_char.clone(),
                        ExprKind::Variable(NamePath::new_plain(param_name)).at(source),
                    )],
                    result_type: void.clone(),
                }))
                .at(source),
            )
            .at(source),
        ],
    )
}

pub fn setup_build_system_interpreter_symbols(file: &mut RawAstFile, is_pragma: bool) {
    let source = Source::internal();
    let void = TypeKind::Void.at(Source::internal());
    let ptr_char = TypeKind::Ptr(Box::new(TypeKind::char().at(source))).at(source);

    // Call to function we actually care about
    let call = ExprKind::Call(Box::new(Call {
        name_path: NamePath::new_plain("main"),
        args: vec![],
        expected_to_return: is_pragma.then(|| void.clone()),
        generics: vec![],
        using: vec![],
    }))
    .at(Source::internal());

    file.funcs.push(Func::new(
        FuncHead {
            name: "<interpreter entry point>".into(),
            type_params: TypeParams::default(),
            givens: vec![],
            params: Params::default(),
            return_type: void.clone(),
            ownership: SymbolOwnership::Owned(Exposure::Hidden),
            source,
            abide_abi: false,
            tag: Some(Tag::InterpreterEntryPoint),
            privacy: Privacy::Public,
        },
        if is_pragma {
            vec![StmtKind::Return(Some(call)).at(Source::internal())]
        } else {
            vec![StmtKind::Expr(call).at(Source::internal())]
        },
    ));

    file.funcs
        .push(thin_cstring_func("println", "message", Syscall::Println));

    if is_pragma {
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
            params: TypeParams::default(),
            fields: IndexMap::from_iter([(
                "kind".into(),
                Field {
                    ast_type: TypeKind::Named(NamePath::new_plain("ProjectKind"), vec![])
                        .at(source),
                    privacy: Privacy::Public,
                    source,
                },
            )]),
            is_packed: false,
            source,
            privacy: Privacy::Private,
        });

        file.structs.push(Struct {
            name: "Dependency".into(),
            params: TypeParams::default(),
            fields: IndexMap::from_iter([(
                "name".into(),
                Field {
                    ast_type: ptr_char.clone(),
                    privacy: Privacy::Private,
                    source,
                },
            )]),
            is_packed: false,
            source,
            privacy: Privacy::Private,
        });

        file.funcs.push(thin_cstring_func(
            "adept",
            "version",
            Syscall::BuildSetAdeptVersion,
        ));

        file.funcs.push(thin_cstring_func(
            "link",
            "filename",
            Syscall::BuildLinkFilename,
        ));

        file.funcs.push(thin_cstring_func(
            "linkFramework",
            "framework_name",
            Syscall::BuildLinkFrameworkName,
        ));

        file.funcs.push(thin_cstring_func(
            "experimental",
            "experiment",
            Syscall::Experimental,
        ));

        file.funcs.push(thin_cstring_func(
            "importNamespace",
            "namespace",
            Syscall::ImportNamespace,
        ));

        file.funcs.push(thin_void_func(
            "dontAssumeIntAtLeast32Bits",
            Syscall::DontAssumeIntAtLeast32Bits,
        ));

        file.funcs.push(Func::new(
            FuncHead {
                name: "project".into(),
                type_params: TypeParams::default(),
                givens: vec![],
                params: Params::normal([
                    Param::named("name".into(), ptr_char.clone()),
                    Param::named(
                        "project".into(),
                        TypeKind::Named(NamePath::new_plain("Project"), vec![]).at(source),
                    ),
                ]),
                return_type: void.clone(),
                abide_abi: false,
                source,
                tag: None,
                privacy: Privacy::Public,
                ownership: SymbolOwnership::default(),
            },
            vec![
                StmtKind::Expr(
                    ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                        kind: Syscall::BuildAddProject,
                        args: vec![
                            (
                                ptr_char.clone(),
                                ExprKind::Variable(NamePath::new_plain("name")).at(source),
                            ),
                            (
                                TypeKind::Named(NamePath::new_plain("ProjectKind"), vec![])
                                    .at(source),
                                ExprKind::Member(
                                    Box::new(
                                        ExprKind::Variable(NamePath::new_plain("project"))
                                            .at(source),
                                    ),
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
                .at(source),
            ],
        ));

        file.funcs.push(Func::new(
            FuncHead {
                name: "use".into(),
                type_params: TypeParams::default(),
                givens: vec![],
                params: Params::normal([
                    Param::named("as_namespace".into(), ptr_char.clone()),
                    Param::named(
                        "dependency".into(),
                        TypeKind::Named(NamePath::new_plain("Dependency"), vec![]).at(source),
                    ),
                ]),
                return_type: void.clone(),
                abide_abi: false,
                source,
                tag: None,
                privacy: Privacy::Public,
                ownership: SymbolOwnership::default(),
            },
            vec![
                StmtKind::Expr(
                    ExprKind::InterpreterSyscall(Box::new(InterpreterSyscall {
                        kind: Syscall::UseDependency,
                        args: vec![
                            (
                                ptr_char.clone(),
                                ExprKind::Variable(NamePath::new_plain("as_namespace")).at(source),
                            ),
                            (
                                TypeKind::Named(NamePath::new_plain("Dependency"), vec![])
                                    .at(source),
                                ExprKind::Member(
                                    Box::new(
                                        ExprKind::Variable(NamePath::new_plain("dependency"))
                                            .at(source),
                                    ),
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
                .at(source),
            ],
        ));

        file.funcs.push(Func::new(
            FuncHead {
                name: "import".into(),
                type_params: TypeParams::default(),
                givens: vec![],
                params: Params::normal([Param::named("namespace".into(), ptr_char.clone())]),
                return_type: TypeKind::Named(NamePath::new_plain("Dependency"), vec![]).at(source),
                abide_abi: false,
                ownership: SymbolOwnership::default(),
                source,
                tag: None,
                privacy: Privacy::Public,
            },
            vec![
                StmtKind::Return(Some(
                    ExprKind::StructLiteral(Box::new(StructLiteral {
                        ast_type: TypeKind::Named(NamePath::new_plain("Dependency"), vec![])
                            .at(source),
                        fields: vec![FieldInitializer {
                            name: None,
                            value: ExprKind::Variable(NamePath::new_plain("namespace")).at(source),
                        }],
                        fill_behavior: FillBehavior::Forbid,
                        language: Language::Adept,
                    }))
                    .at(source),
                ))
                .at(source),
            ],
        ));
    }
}

pub fn run_build_system_interpreter<'a>(
    ir_module: &'a ir::Module,
) -> Result<Interpreter<'a, BuildSystemSyscallHandler>, InterpreterError> {
    let interpreter_entry_point = ir_module
        .interpreter_entry_point
        .ok_or_else(|| InterpreterError::PolymorphicEntryPoint)?;

    let max_steps = Some(1_000_000);
    let handler = BuildSystemSyscallHandler::default();
    let mut interpreter = Interpreter::new(handler, ir_module, max_steps);

    let result = interpreter.run(interpreter_entry_point)?;

    assert!(result.kind.is_literal() && result.kind.unwrap_literal().is_void());
    Ok(interpreter)
}
