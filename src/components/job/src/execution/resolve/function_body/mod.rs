mod basic_bin_op;
mod dominators;

use crate::{
    BasicBlockId, CfgBuilder, Continuation, EndInstrKind, Executable, ExecutionCtx, Executor,
    FuncSearch, InstrKind, RevPostOrderIterWithEnds, Suspend, SuspendMany, are_types_equal,
    conform::{ConformMode, conform_to, conform_to_default},
    execution::resolve::ResolveType,
    flatten_func,
    module_graph::ModuleView,
    repr::{
        Compiler, DeclHead, FuncBody, FuncHead, Mutability, Type, TypeKind, UnaliasedType,
        Variable, Variables,
    },
    unify::unify_types,
};
use arena::ArenaMap;
use ast::Integer;
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use dominators::{PostOrder, compute_idom_tree};
use primitives::{CInteger, CIntegerAssumptions};

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveFunctionBody<'env> {
    view: ModuleView<'env>,
    func: ByAddress<&'env ast::Func>,
    resolved_head: ByAddress<&'env FuncHead<'env>>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_types: SuspendMany<'env, &'env Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    cfg: Option<CfgBuilder<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    dominators_and_post_order: Option<(ArenaMap<BasicBlockId, BasicBlockId>, PostOrder)>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    variables: Option<Variables<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    rev_post_order: Option<RevPostOrderIterWithEnds>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_type: Suspend<'env, UnaliasedType<'env>>,
}

impl<'env> ResolveFunctionBody<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        func: &'env ast::Func,
        resolved_head: &'env FuncHead<'env>,
    ) -> Self {
        Self {
            view,
            func: ByAddress(func),
            resolved_head: ByAddress(resolved_head),
            inner_types: None,
            rev_post_order: None,
            resolved_type: None,
            compiler: ByAddress(compiler),
            cfg: None,
            dominators_and_post_order: None,
            variables: None,
        }
    }
}

/*
- For both implementation simplicity and mental processing simplicity, we will forbid gotos, break, and continue within expression aliases
    - Macros can be used if non-trivial control flow modifications are necessary, e.g. @await or @yield or @break(2) or @try etc.

#### Function Body Resolution Order
 - Flatten AST to CFG nodes and GOTO relocations
 - Resolve macros to CFG flows
 - Resolve GOTO relocations
 - Determine post order traversal
 - Determine immediate dominators tree
 - Resolve variable names to corresponding nodes
 - Any unresolved variable names are global variables or expression aliases.
   - If global variable, the process is trivial.
   - If an expression alias, we have to inject the flattened version in place of the variable usage.
     - We must forbid any control flow that doesn't stay within the range / return.
       For example, `goto`, `break`, `continue`, and `@label@` are not allowed as they don't operate "as sequential values" but rather non-trivially alter control flow.
       We probably want to treat `break` and `continue` as if they were unnamed goto labels.
       I think we also need to link them after macro resolution so that they will still work even if generated from macros.
*/

impl<'env> Executable<'env> for ResolveFunctionBody<'env> {
    type Output = &'env FuncBody<'env>;

    #[allow(unused_variables)]
    #[allow(unreachable_code)]
    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let def = self.func;
        let builtin_types = self.compiler.builtin_types;

        // 0) Foreign functions do not have bodies
        assert!(def.head.ownership.is_owned());

        // 1) Compute control flow graph
        let cfg = match self.cfg.as_mut() {
            Some(cfg) => cfg,
            None => self.cfg.insert(
                flatten_func(ctx, &def.head.params, &def.stmts, def.head.source)
                    .finalize_gotos()?,
            ),
        };

        // 2) Compute immediate dominators and post order traversal
        let (dominators, post_order) = self
            .dominators_and_post_order
            .get_or_insert_with(|| compute_idom_tree(&cfg));

        // 3) Acquire settings and configuration
        //let settings = &self.settings[def.settings.expect("settings assigned for function")];
        //let c_integer_assumptions = settings.c_integer_assumptions();
        let c_integer_assumptions = CIntegerAssumptions::default();

        let variables = self.variables.get_or_insert_default();

        let rev_post_order = self
            .rev_post_order
            .get_or_insert_with(|| RevPostOrderIterWithEnds::new(post_order));

        // 5) Resolve types and linked data for each CFG node (may suspend)
        while let Some(instr_ref) = rev_post_order.peek() {
            let bb = cfg.get_unsafe(instr_ref.basicblock);

            // If we're processing an end instruction, we need to handle it separately
            if instr_ref.instr_or_end >= bb.inner_len() {
                let instr = &bb.end.as_ref().unwrap();

                match &instr.kind {
                    EndInstrKind::IncompleteGoto(_)
                    | EndInstrKind::IncompleteBreak
                    | EndInstrKind::IncompleteContinue => unreachable!(),
                    EndInstrKind::Return(Some(value), _) => {
                        // 1] Conform the value to return type
                        let conformed = conform_to(
                            ctx,
                            cfg.get_typed(*value),
                            self.resolved_head.return_type,
                            c_integer_assumptions,
                            builtin_types,
                            self.view.target(),
                            ConformMode::Normal,
                            |_, _| todo!("handle polymorphs"),
                            instr.source,
                        );

                        // 2] Ensure the type of the value matches the return type
                        let Some(conformed) = conformed else {
                            return Err(ErrorDiagnostic::new(
                                format!(
                                    "Cannot return value of type '{}', expected '{}'",
                                    cfg.get_typed(*value).0,
                                    self.resolved_head.return_type.0
                                ),
                                instr.source,
                            )
                            .into());
                        };

                        // 3] Track any casts that were necessary
                        cfg.set_primary_unary_cast(instr_ref, conformed.cast);
                    }
                    EndInstrKind::Return(None, _) => {
                        if !self.resolved_head.return_type.0.kind.is_void() {
                            return Err(ErrorDiagnostic::new(
                                format!(
                                    "Must return a value of type '{}' before exiting function '{}'",
                                    self.resolved_head.return_type, self.resolved_head.name
                                ),
                                instr.source,
                            )
                            .into());
                        }
                    }
                    EndInstrKind::Jump(..)
                    | EndInstrKind::Branch(..)
                    | EndInstrKind::NewScope(..)
                    | EndInstrKind::Unreachable => (),
                }

                rev_post_order.next_partial_block(cfg, post_order);
                continue;
            }

            // Otherwise, we're processing a normal instruction
            let instr = &bb.instrs[instr_ref.instr_or_end as usize];

            match &instr.kind {
                InstrKind::Phi(items, conform_behavior) if items.len() <= 1 => {
                    // 0] We don't need to do anything fancy if there's not more
                    //    than one incoming value.
                    let instr = items.first().unwrap().1;

                    let typed = instr
                        .map(|instr_ref| cfg.get_typed(instr_ref))
                        .unwrap_or(builtin_types.never());

                    cfg.set_typed(instr_ref, typed)
                }
                InstrKind::Phi(items, conform_behavior) => {
                    // NOTE: We already handled the case for when `items.len() <= 1` above.

                    // 0] Determine types
                    let types_iter = items
                        .iter()
                        .flat_map(|(bb, value)| value.map(|value| cfg.get_typed(value)));

                    // 1] Determine unifying type
                    let unified = unify_types(
                        ctx,
                        types_iter,
                        conform_behavior.expect("conform behavior for >2 incoming phi"),
                        builtin_types,
                        instr.source,
                    );

                    // 2] Set incoming jumps to conform to the unifying type
                    if let Some(unified) = unified {
                        for (bb, value) in items.iter() {
                            if value.is_some() {
                                cfg.set_pre_jump_typed_unary_cast(
                                    *bb,
                                    todo!("optional unary casts for incoming edges for CFG phi"),
                                );
                            }
                        }
                    }

                    // 3] Result type is the unified type
                    cfg.set_typed(instr_ref, unified.unwrap_or(builtin_types.never()))
                }
                InstrKind::Name(needle, _) => {
                    // 1] Start by checking within current basicblock
                    let mut dom_ref = instr_ref.basicblock;
                    let mut num_take = instr_ref.instr_or_end as usize;

                    // 2] Go up through the immediate dominators, checking
                    //    in reverse instruction order to find the associated
                    //    declaration.
                    //    We may want to accelerate this later on using an
                    //    auxiliary data structure, especially for larger blocks.
                    let found = 'outer: loop {
                        let dom = &cfg.get_unsafe(dom_ref);

                        // Loop through the instructions in reverse order to find declaration
                        for (instr_index, instr) in
                            dom.instrs.iter().take(num_take).enumerate().rev()
                        {
                            if let Some(found) = match &instr.kind {
                                InstrKind::Parameter(name, _, _, variable_ref)
                                | InstrKind::Declare(name, _, _, _, variable_ref)
                                | InstrKind::DeclareAssign(name, _, _, variable_ref)
                                    if name == needle =>
                                {
                                    *variable_ref
                                }
                                _ => None,
                            } {
                                break 'outer found;
                            }
                        }

                        // Otherwise, we have to look in the next immediate dominator.
                        let next_dom_ref = *dominators.get(dom_ref).unwrap();

                        // If we dominate ourselves, then there's nowhere left to look
                        // within the function.
                        if dom_ref == next_dom_ref {
                            return Err(ErrorDiagnostic::new(
                                format!("Undeclared variable '{}'", needle),
                                instr.source,
                            )
                            .into());
                        }

                        // Advance up the immediate dominator tree.
                        dom_ref = next_dom_ref;
                        num_take = usize::MAX;
                    };

                    // 3] Extract the type of the found variable and create deref'T
                    let variable_type = variables.get(found).ty;
                    let deref_variable_type = UnaliasedType(ctx.alloc(
                        TypeKind::Deref(variable_type.0, Mutability::Mutable).at(instr.source),
                    ));

                    // 4] Set result
                    cfg.set_typed(instr_ref, deref_variable_type);
                    cfg.set_variable_ref(instr_ref, found);
                }
                InstrKind::Parameter(_, ast_type, _, _) => {
                    // 0] Resolve variable type
                    let Some(resolved_type) = executor.demand(self.resolved_type) else {
                        return suspend!(
                            self.resolved_type,
                            executor.request(ResolveType::new(self.view, ast_type)),
                            ctx
                        );
                    };

                    // 1] Assign the variable the resolved type
                    let variable_ref = variables.push(Variable::new(resolved_type));
                    cfg.set_variable_ref(instr_ref, variable_ref);

                    // 2] Reset the "suspend on type" for the next time we need it
                    self.resolved_type = None;

                    // 3] Finish typing the paramater declaration, which itself is void
                    cfg.set_typed(instr_ref, builtin_types.void())
                }
                InstrKind::Declare(name, ast_type, value, _, _) => {
                    // 1] Resolve variable type
                    let Some(resolved_type) = executor.demand(self.resolved_type) else {
                        return suspend!(
                            self.resolved_type,
                            executor.request(ResolveType::new(self.view, ast_type)),
                            ctx
                        );
                    };

                    // 2] Ensure variable has initial value
                    let Some(value) = value else {
                        return Err(ErrorDiagnostic::new(
                            format!("Variable '{}' must have an initial value", name),
                            instr.source,
                        )
                        .into());
                    };

                    // 3] Conform the variable's initial value to its default concrete type
                    let Some(conformed) = conform_to(
                        ctx,
                        cfg.get_typed(*value),
                        resolved_type,
                        c_integer_assumptions,
                        builtin_types,
                        self.view.target(),
                        ConformMode::Normal,
                        |_, _| todo!("handle polymorphs"),
                        instr.source,
                    ) else {
                        return Err(ErrorDiagnostic::new(
                            format!(
                                "Incompatible types '{}' and '{}'",
                                cfg.get_typed(*value),
                                resolved_type
                            ),
                            instr.source,
                        )
                        .into());
                    };

                    // 4] Assign the variable the resolved type, and
                    //    track the final type along with any necessary casts.
                    let variable_ref = variables.push(Variable { ty: resolved_type });
                    cfg.set_variable_ref(instr_ref, variable_ref);
                    cfg.set_primary_unary_cast(instr_ref, conformed.cast);
                    cfg.set_typed(instr_ref, builtin_types.void());

                    // 5] Reset the "suspend on type" for the next time we need it
                    self.resolved_type = None;
                }
                InstrKind::Assign(instr_ref, instr_ref1) => todo!("assign"),
                InstrKind::BinOp(instr_ref, basic_binary_operator, instr_ref1, language) => {
                    todo!()
                }
                InstrKind::BooleanLiteral(value) => {
                    cfg.set_typed(
                        instr_ref,
                        UnaliasedType(ctx.alloc(TypeKind::BooleanLiteral(*value).at(instr.source))),
                    );
                }
                InstrKind::IntegerLiteral(integer) => match integer {
                    Integer::Known(known) => {
                        todo!("conform known type integer literal to final type")
                    }
                    Integer::Generic(big_int) => {
                        cfg.set_typed(
                            instr_ref,
                            UnaliasedType(
                                ctx.alloc(TypeKind::IntegerLiteral(big_int).at(instr.source)),
                            ),
                        );
                    }
                },
                InstrKind::FloatLiteral(floating) => {
                    cfg.set_typed(
                        instr_ref,
                        UnaliasedType(ctx.alloc(
                            TypeKind::FloatLiteral((*floating).try_into().ok()).at(instr.source),
                        )),
                    );
                }
                InstrKind::AsciiCharLiteral(c) => {
                    cfg.set_typed(
                        instr_ref,
                        UnaliasedType(ctx.alloc(TypeKind::AsciiCharLiteral(*c).at(instr.source))),
                    );
                }
                InstrKind::Utf8CharLiteral(_) => todo!("utf-8 char literal"),
                InstrKind::StringLiteral(_) => todo!("string literal"),
                InstrKind::NullTerminatedStringLiteral(cstr) => {
                    cfg.set_typed(
                        instr_ref,
                        UnaliasedType(
                            ctx.alloc(
                                TypeKind::Ptr(ctx.alloc(
                                    TypeKind::CInteger(CInteger::Char, None).at(instr.source),
                                ))
                                .at(instr.source),
                            ),
                        ),
                    )
                }
                InstrKind::NullLiteral => {
                    cfg.set_typed(instr_ref, builtin_types.null());
                }
                InstrKind::VoidLiteral => {
                    cfg.set_typed(instr_ref, builtin_types.void());
                }
                InstrKind::Call(call, _) => {
                    let symbol = match self.view.find_symbol(
                        executor,
                        FuncSearch {
                            name: call.name,
                            source: instr.source,
                        },
                    ) {
                        Ok(symbol) => symbol,
                        Err(into_continuation) => return Err(into_continuation(self.into())),
                    };

                    let DeclHead::FuncLike(func_head) = symbol else {
                        return Err(
                            ErrorDiagnostic::new("Cannot call non-function", instr.source).into(),
                        );
                    };

                    let mut arg_casts = vec![];
                    let mut variadic_arg_types = vec![];

                    // WARNING: This part assumes we don't suspend
                    for (i, arg) in call.args.iter().enumerate() {
                        let got_type = cfg.get_typed(*arg);

                        let conformed = if let Some(param) = func_head.params.required.get(i) {
                            let expected_type = param.ty;

                            // Conform the value to parameter type if present
                            // WARNING: This part assumes we don't suspend
                            let Some(conformed) = conform_to(
                                ctx,
                                got_type,
                                expected_type,
                                c_integer_assumptions,
                                builtin_types,
                                self.view.target(),
                                ConformMode::ParameterPassing,
                                |_, _| todo!("handle polymorphs"),
                                instr.source,
                            ) else {
                                return Err(ErrorDiagnostic::new(
                                    format!(
                                        "Expected '{}' for argument #{}, but got '{}'",
                                        expected_type,
                                        i + 1,
                                        got_type
                                    ),
                                    instr.source,
                                )
                                .into());
                            };

                            conformed
                        } else {
                            // Conform variadic arguments to their default types
                            // WARNING: This part assumes we don't suspend
                            let conformed = conform_to_default(
                                ctx,
                                got_type,
                                c_integer_assumptions,
                                builtin_types,
                                self.view.target(),
                            )?;

                            variadic_arg_types.push(conformed.ty);
                            conformed
                        };

                        arg_casts.push(conformed.cast);
                    }

                    let arg_casts = ctx.alloc_slice_fill_iter(arg_casts.into_iter());
                    let variadic_arg_types =
                        ctx.alloc_slice_fill_iter(variadic_arg_types.into_iter());

                    assert_eq!(arg_casts.len(), call.args.len());
                    assert_eq!(
                        variadic_arg_types.len(),
                        call.args.len() - func_head.params.required.len()
                    );

                    cfg.set_typed_and_callee(instr_ref, func_head, arg_casts, variadic_arg_types);
                }
                InstrKind::DeclareAssign(_, value, _, _) => {
                    // 1] Conform the value to its default concrete type
                    let conformed = conform_to_default(
                        ctx,
                        cfg.get_typed(*value),
                        c_integer_assumptions,
                        builtin_types,
                        self.view.target(),
                    )?;

                    // 2] Assign the variable the resolved type, and
                    //    track the final type along with any necessary casts.
                    let variable_ref = variables.push(Variable::new(conformed.ty));
                    cfg.set_variable_ref(instr_ref, variable_ref);
                    cfg.set_primary_unary_cast(instr_ref, conformed.cast);
                    cfg.set_typed(instr_ref, conformed.ty);
                }
                InstrKind::Member(instr_ref, _, privacy) => todo!("member"),
                InstrKind::ArrayAccess(instr_ref, instr_ref1) => todo!("array access"),
                InstrKind::StructLiteral(struct_literal_instr) => todo!("struct litereal"),
                InstrKind::UnaryOperation(unary_operator, instr_ref) => todo!("unary operation"),
                InstrKind::SizeOf(_, size_of_mode) => todo!("sizeof"),
                InstrKind::SizeOfValue(instr_ref, size_of_mode) => todo!("sizeof value"),
                InstrKind::InterpreterSyscall(interpreter_syscall_instr) => {
                    todo!("interpreter syscall")
                }
                InstrKind::IntegerPromote(instr_ref) => todo!("integer promote"),
                InstrKind::ConformToBool(value, language, _) => {
                    let conformed = conform_to_default(
                        ctx,
                        cfg.get_typed(*value),
                        c_integer_assumptions,
                        builtin_types,
                        self.view.target(),
                    )?;

                    if !conformed.ty.0.kind.is_boolean() {
                        return Err(
                            ErrorDiagnostic::new("Expected 'bool' value", instr.source).into()
                        );
                    }

                    cfg.set_typed(instr_ref, conformed.ty);
                    cfg.set_primary_unary_cast(instr_ref, conformed.cast);
                }
                InstrKind::Is(instr_ref, _) => todo!("is"),
                InstrKind::LabelLiteral(_) => todo!("label literal"),
            }

            rev_post_order.next_partial_block(cfg, post_order);
        }

        /*
        while self.num_resolved_nodes < post_order.len() {
            // We must process nodes in reverse post-order to ensure proper ordering, lexical
            // ordering is not enough! Consider `goto` for example.
            let node_ref = post_order[post_order.len() - 1 - self.num_resolved_nodes];
            let node = &cfg.nodes[node_ref];

            let get_typed = |node_ref: NodeRef| -> &Resolved<'env> {
                return self.resolved_nodes.get(node_ref.into_raw()).unwrap();
            };

            let resolved_node = match &node.kind {
                NodeKind::Start(_) => Resolved::from_type(builtin_types.void()),
                NodeKind::Sequential(sequential_node) => match &sequential_node.kind {
                    SequentialNodeKind::Join1(incoming) => get_typed(*incoming).clone(),
                    SequentialNodeKind::JoinN(items, Some(conform_behavior)) => {
                        let edges = items
                            .iter()
                            .map(|(_, incoming)| Value::new(*incoming))
                            .collect_vec();

                        let Some(unified) = unify_types(
                            ctx,
                            None,
                            edges
                                .iter()
                                .map(|value| value.ty(&self.resolved_nodes, builtin_types)),
                            *conform_behavior,
                            builtin_types,
                            node.source,
                        ) else {
                            // TODO: Improve error message
                            return Err(ErrorDiagnostic::new(
                                format!("Inconsistent types from incoming blocks"),
                                node.source,
                            )
                            .into());
                        };

                        Resolved::from_type(unified)
                    }
                    SequentialNodeKind::JoinN(items, None) => {
                        Resolved::from_type(builtin_types.void())
                    }
                    SequentialNodeKind::Const(_) => {
                        // For this case, we will have to suspend until the result
                        // of the calcuation is complete
                        unimplemented!("SequentialNodeKind::Const not supported yet")
                    }
                    SequentialNodeKind::Name(needle) => {
                        let mut dominator_ref = *dominators
                            .get(node_ref.into_raw())
                            .expect("dominator to exist");

                        let found = loop {
                            let dom = &cfg.nodes[dominator_ref];

                            match &dom.kind {
                                NodeKind::Start(_) => {
                                    return Err(ErrorDiagnostic::new(
                                        format!("Undeclared variable '{}'", needle),
                                        node.source,
                                    )
                                    .into());
                                }
                                NodeKind::Sequential(SequentialNode {
                                    kind:
                                        SequentialNodeKind::Parameter(name, _, _)
                                        | SequentialNodeKind::Declare(name, _, _)
                                        | SequentialNodeKind::DeclareAssign(name, _),
                                    ..
                                }) if needle.as_plain_str() == Some(name) => {
                                    break dominator_ref;
                                }
                                _ => (),
                            }

                            dominator_ref = *dominators.get(dominator_ref.into_raw()).unwrap();
                        };

                        let var_ty = variables
                            .get(found)
                            .expect("variable to be tracked")
                            .ty
                            .expect("variable to have had type resolved");

                        eprintln!("Variable {} has type {:?}", needle, var_ty);
                        Resolved::from_type(var_ty.clone())
                    }
                    SequentialNodeKind::Declare(_, ast_type, _)
                    | SequentialNodeKind::Parameter(_, ast_type, _) => {
                        // Resolve variable type
                        let Some(resolved_type) = executor.demand(self.resolved_type) else {
                            return suspend!(
                                self.resolved_type,
                                executor.request(ResolveType::new(self.view, ast_type,)),
                                ctx
                            );
                        };

                        variables.assign_resolved_type(node_ref, resolved_type);

                        // Reset suspend on type for next node that needs it
                        self.resolved_type = None;

                        Resolved::from_type(builtin_types.void())
                    }
                    SequentialNodeKind::Assign(_, _) => Resolved::from_type(builtin_types.void()),
                    SequentialNodeKind::BinOp(left, bin_op, right, language) => {
                        let left_ty = get_typed(*left).ty();
                        let right_ty = get_typed(*right).ty();

                        if let (TypeKind::IntegerLiteral(left), TypeKind::IntegerLiteral(right)) =
                            (&left_ty.0.kind, &right_ty.0.kind)
                        {
                            resolve_basic_binary_operation_expr_on_literals(
                                ctx,
                                bin_op,
                                &left,
                                &right,
                                node.source,
                            )
                            .map_err(Continuation::Error)?
                        } else {
                            let conform_behavior = match language {
                                ast::Language::Adept => {
                                    ConformBehavior::Adept(c_integer_assumptions)
                                }
                                ast::Language::C => ConformBehavior::C,
                            };

                            let preferred_type = preferred_types.get(node_ref.into_raw()).copied();

                            let Some(unified_type) = unify_types(
                                ctx,
                                preferred_type,
                                [left_ty, right_ty].into_iter(),
                                conform_behavior,
                                builtin_types,
                                node.source,
                            ) else {
                                return Err(ErrorDiagnostic::new(
                                    format!(
                                        "Incompatible types '{}' and '{}' for '{}'",
                                        left_ty, right_ty, bin_op
                                    ),
                                    node.source,
                                )
                                .into());
                            };

                            let operator =
                                resolve_basic_binary_operator(bin_op, unified_type, node.source)
                                    .map_err(Continuation::Error)?;

                            dbg!(&preferred_type, &unified_type, &operator);
                            let result_type = if bin_op.returns_boolean() {
                                builtin_types.bool()
                            } else {
                                unified_type
                            };

                            Resolved::new(result_type, ResolvedData::BinaryImplicitCast(operator))
                        }
                    }
                    SequentialNodeKind::Boolean(value) => Resolved::from_type(UnaliasedType(
                        ctx.alloc(TypeKind::BooleanLiteral(*value).at(node.source)),
                    )),
                    SequentialNodeKind::Integer(integer) => {
                        let source = node.source;

                        let ty = match integer {
                            ast::Integer::Known(known) => TypeKind::from(known.as_ref()).at(source),
                            ast::Integer::Generic(value) => {
                                TypeKind::IntegerLiteral(value.clone()).at(source)
                            }
                        };

                        Resolved::from_type(UnaliasedType(ctx.alloc(ty)))
                    }
                    SequentialNodeKind::Float(_) => todo!("Float"),
                    SequentialNodeKind::AsciiChar(_) => todo!("AsciiChar"),
                    SequentialNodeKind::Utf8Char(_) => todo!("Utf8Char"),
                    SequentialNodeKind::String(_) => todo!("String"),
                    SequentialNodeKind::NullTerminatedString(cstring) => {
                        let char = TypeKind::CInteger(CInteger::Char, None).at(node.source);
                        Resolved::from_type(UnaliasedType(
                            ctx.alloc(TypeKind::Ptr(ctx.alloc(char)).at(node.source)),
                        ))
                    }
                    SequentialNodeKind::Null => todo!("Null"),
                    SequentialNodeKind::Void => Resolved::from_type(builtin_types.void()),
                    SequentialNodeKind::Call(node_call) => {
                        todo!("call expressions are not supported yet by ResolveFunctionBody")
                        // let found = find_or_suspend();

                        // let all_conformed = existing_conformed.conform_rest(rest).or_suspend();

                        // result
                    }
                    SequentialNodeKind::DeclareAssign(_name, value) => {
                        let declared = conform_to_default(
                            ctx,
                            get_typed(*value).ty(),
                            c_integer_assumptions,
                            builtin_types,
                        )?;
                        variables.assign_resolved_type(node_ref, declared.ty());
                        declared
                    }
                    SequentialNodeKind::Member(idx, _, privacy) => todo!("Member"),
                    SequentialNodeKind::ArrayAccess(idx, idx1) => todo!("ArrayAccess"),
                    SequentialNodeKind::StructLiteral(node_struct_literal) => {
                        todo!("StructLiteral")
                    }
                    SequentialNodeKind::UnaryOperation(unary_operator, idx) => {
                        todo!("UnaryOperation")
                    }
                    SequentialNodeKind::StaticMemberValue(static_member_value) => {
                        todo!("StaticMemberValue")
                    }
                    SequentialNodeKind::StaticMemberCall(node_static_member_call) => {
                        todo!("StaticMemberCall")
                    }
                    SequentialNodeKind::SizeOf(_, _) => todo!("SizeOf"),
                    SequentialNodeKind::SizeOfValue(_, _) => todo!("SizeOfValue"),
                    SequentialNodeKind::InterpreterSyscall(node_interpreter_syscall) => {
                        todo!("InterpreterSyscall")
                    }
                    SequentialNodeKind::IntegerPromote(idx) => todo!("IntegerPromote"),
                    SequentialNodeKind::StaticAssert(idx, _) => todo!("StaticAssert"),
                    SequentialNodeKind::ConformToBool(idx, language) => {
                        if let ast::Language::Adept = language {
                            Resolved::from_type(builtin_types.bool())
                        } else {
                            todo!("ConformToBool")
                        }
                    }
                    SequentialNodeKind::Is(idx, _) => unimplemented!("Is"),
                    SequentialNodeKind::DirectGoto(label_value) => {
                        Resolved::from_type(builtin_types.never())
                    }
                    SequentialNodeKind::LabelLiteral(name) => {
                        return Err(ErrorDiagnostic::new(
                            "Indirect goto labels are not supported yet",
                            node.source,
                        )
                        .into());
                    }
                },
                NodeKind::Branching(branch_node) => Resolved::from_type(builtin_types.void()),
                NodeKind::Terminating(terminating_node) => {
                    Resolved::from_type(builtin_types.never())
                }
                NodeKind::Scope(_) => {
                    // When joining control flow paths, consider the path immediately
                    // from this "begin scope" node to diverge since it's not a real branch.
                    // Since it's only used for scoping, so don't take it into account
                    // during type unifying from multiple code paths.
                    Resolved::from_type(builtin_types.never())
                }
            };

            self.resolved_nodes
                .insert(node_ref.into_raw(), resolved_node);
            self.num_resolved_nodes += 1;
        }
        */

        let final_cfg = ctx.alloc(self.cfg.take().unwrap().finish(ctx));

        Ok(ctx.alloc(FuncBody {
            cfg: final_cfg,
            post_order: ctx.alloc_slice_fill_iter(std::mem::take(post_order).iter().copied()),
            variables: std::mem::take(variables),
        }))
    }
}
