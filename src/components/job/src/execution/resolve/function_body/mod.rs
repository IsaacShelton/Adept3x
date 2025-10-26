mod basic_bin_op;
mod dominators;

use crate::{
    BasicBlockId, CfgBuilder, CfgValue, Continuation, EndInstrKind, Executable, ExecutionCtx,
    Executor, FuncSearch, InstrKind, RevPostOrderIterWithEnds, Suspend, SuspendMany,
    conform::{ConformMode, UnaryCast, conform_to, conform_to_default},
    execution::resolve::{ResolveNamespace, ResolveType, structure::ResolveStructureBody},
    flatten_func,
    module_graph::ModuleView,
    repr::{
        Compiler, DeclHead, FuncBody, FuncHead, StructBody, Type, TypeDisplayerDisambiguation,
        TypeHeadRest, TypeHeadRestKind, TypeKind, UnaliasedType, UserDefinedType, Variable,
        Variables,
    },
    sub_task::SubTask,
    unify::unify_types,
};
use arena::ArenaMap;
use ast::{ConformBehavior, Integer};
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use dominators::{PostOrder, compute_idom_tree};
use itertools::Itertools;
use primitives::{CInteger, CIntegerAssumptions};
use std::collections::HashSet;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveFunctionBody<'env> {
    view: &'env ModuleView<'env>,
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

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_namespace: Option<ResolveNamespace<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_struct_body: Suspend<'env, &'env StructBody<'env>>,
}

impl<'env> ResolveFunctionBody<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
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
            resolved_namespace: None,
            resolved_struct_body: None,
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
                    EndInstrKind::Return(value, _) => {
                        // 1] Conform the value to return type
                        let conformed = conform_to(
                            ctx,
                            cfg.get_typed(*value, builtin_types),
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
                                if matches!(value, CfgValue::Void) && !self.resolved_head.return_type.0.kind.is_void() {
                                    format!(
                                        "Must return a value of type `{}` before exiting function `{}`",
                                        self.resolved_head.return_type.display_one(self.view), self.resolved_head.name
                                    )
                                } else {
                                    let expected = self.resolved_head.return_type;
                                    let got = cfg.get_typed((*value).into(), builtin_types);
                                    let disambiguation = TypeDisplayerDisambiguation::new([got.0, expected.0].into_iter());

                                    format!(
                                        "Cannot return value of type `{}`, expected `{}`",
                                        got.display(self.view, &disambiguation),
                                        expected.display(self.view, &disambiguation)
                                    )
                                },
                                instr.source,
                            )
                            .into());
                        };

                        // 3] Track any casts that were necessary
                        cfg.set_primary_unary_cast(instr_ref, conformed.cast);
                    }
                    EndInstrKind::Jump(..)
                    | EndInstrKind::Branch(..)
                    | EndInstrKind::NewScope(..)
                    | EndInstrKind::Unreachable => (),
                }

                rev_post_order.next_in_builder(cfg, post_order);
                continue;
            }

            // Otherwise, we're processing a normal instruction
            let instr = &bb.instrs[instr_ref.instr_or_end as usize];

            match &instr.kind {
                InstrKind::Phi {
                    possible_incoming,
                    conform_behavior: _,
                } if possible_incoming.len() <= 1 => {
                    // 0] We don't need to do anything fancy if there's not more
                    //    than one incoming value.
                    let incoming_value = possible_incoming.first().unwrap().1;
                    let typed = cfg.get_typed(incoming_value, builtin_types);
                    cfg.set_typed(instr_ref, typed)
                }
                InstrKind::Phi {
                    possible_incoming,
                    conform_behavior,
                } => {
                    // NOTE: We already handled the case for when `items.len() <= 1` above.

                    // 0] Determine types
                    let types_iter = possible_incoming
                        .iter()
                        .map(|(bb_id, value)| (*bb_id, cfg.get_typed(*value, builtin_types)));

                    // 1] Determine unifying type
                    let Some(unified) = unify_types(
                        ctx,
                        types_iter.clone(),
                        conform_behavior.expect("conform behavior for >2 incoming phi"),
                        builtin_types,
                        self.view,
                        instr.source,
                    )?
                    else {
                        let disambiguation = TypeDisplayerDisambiguation::new(
                            types_iter.clone().map(|(_, ty)| ty.0),
                        );

                        return Err(ErrorDiagnostic::new(
                            format!(
                                "Cannot merge incompatible types: {}",
                                types_iter
                                    .map(|(_, ty)| ty)
                                    .unique()
                                    .map(|ty| format!(
                                        "`{}`",
                                        ty.display(self.view, &disambiguation)
                                    ))
                                    .join(", ")
                            ),
                            instr.source,
                        )
                        .into());
                    };

                    // 2] Set incoming jumps to conform to the unifying type
                    for (bb, _value) in possible_incoming.iter() {
                        cfg.set_pre_jump_typed_unary_cast(*bb, None, unified.unified);
                    }

                    for (bb, cast) in unified.unary_casts {
                        cfg.set_pre_jump_typed_unary_cast(bb, Some(cast), unified.unified);
                    }

                    // 3] Result type is the unified type
                    cfg.set_typed(instr_ref, unified.unified);
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
                                format!("Undeclared variable `{}`", needle),
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
                    let deref_variable_type =
                        UnaliasedType(ctx.alloc(TypeKind::Deref(variable_type.0).at(instr.source)));

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
                            format!("Variable `{}` must have an initial value", name),
                            instr.source,
                        )
                        .into());
                    };

                    // 3] Conform the variable's initial value to its default concrete type
                    let Some(conformed) = conform_to(
                        ctx,
                        cfg.get_typed(*value, builtin_types),
                        resolved_type,
                        c_integer_assumptions,
                        builtin_types,
                        self.view.target(),
                        ConformMode::Normal,
                        |_, _| todo!("handle polymorphs"),
                        instr.source,
                    ) else {
                        let got = cfg.get_typed(*value, builtin_types);
                        let expected = resolved_type;
                        let disambiguation =
                            TypeDisplayerDisambiguation::new([expected.0, got.0].into_iter());

                        return Err(ErrorDiagnostic::new(
                            format!(
                                "Incompatible types `{}` and `{}`",
                                got.display(self.view, &disambiguation),
                                expected.display(self.view, &disambiguation),
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
                }
                InstrKind::IntoDest(dest, _) => {
                    let mut dest_ty = cfg.get_typed(*dest, builtin_types).0;
                    let mut unary_cast = None;

                    loop {
                        let TypeKind::Deref(inner_ty) = dest_ty.kind else {
                            break;
                        };

                        let TypeKind::Deref(nested_ty) = inner_ty.kind else {
                            break;
                        };

                        dest_ty = inner_ty;
                        unary_cast = Some(UnaryCast::Dereference {
                            after_deref: UnaliasedType(inner_ty),
                            then: ctx.alloc(unary_cast).as_ref(),
                        });
                    }

                    cfg.set_typed(instr_ref, UnaliasedType(dest_ty));
                    cfg.set_primary_unary_cast(instr_ref, unary_cast);
                }
                InstrKind::Assign {
                    dest,
                    src,
                    src_cast: _,
                } => {
                    let dest_ty = cfg.get_typed(*dest, builtin_types);

                    // NOTE: Assigment itself does not handle nested deref'deref'...,
                    // this is handled by another instruction.

                    let TypeKind::Deref(dest_ty) = cfg.get_typed(*dest, builtin_types).0.kind
                    else {
                        return Err(ErrorDiagnostic::new(
                            format!("Left side of assignment is not mutable"),
                            instr.source,
                        )
                        .into());
                    };

                    let dest_ty = UnaliasedType(dest_ty);

                    let Some(conformed) = conform_to(
                        ctx,
                        cfg.get_typed(*src, builtin_types),
                        dest_ty,
                        c_integer_assumptions,
                        builtin_types,
                        self.view.target(),
                        ConformMode::Normal,
                        |_, _| todo!("handle polymorphs"),
                        instr.source,
                    ) else {
                        let got = cfg.get_typed(*src, builtin_types);
                        let expected = dest_ty;
                        let disambiguation =
                            TypeDisplayerDisambiguation::new([expected.0, got.0].into_iter());

                        return Err(ErrorDiagnostic::new(
                            format!(
                                "Incompatible types `{}` and `{}`",
                                got.display(self.view, &disambiguation),
                                expected.display(self.view, &disambiguation),
                            ),
                            instr.source,
                        )
                        .into());
                    };

                    cfg.set_primary_unary_cast(instr_ref, conformed.cast);
                    cfg.set_typed(instr_ref, builtin_types.void());
                }
                InstrKind::BinOp(a, op, b, conform_behavior, _, _) => {
                    let a_ty = cfg.get_typed(*a, builtin_types);
                    let b_ty = cfg.get_typed(*b, builtin_types);

                    let types_iter = [(false, a_ty), (true, b_ty)].into_iter();

                    let Some(unified) = unify_types(
                        ctx,
                        types_iter.clone(),
                        *conform_behavior,
                        builtin_types,
                        self.view,
                        instr.source,
                    )?
                    else {
                        let disambiguation = TypeDisplayerDisambiguation::new(
                            types_iter.clone().map(|(_, ty)| ty.0),
                        );

                        return Err(ErrorDiagnostic::new(
                            format!(
                                "Cannot {} incompatible types {}",
                                op.verb(),
                                types_iter
                                    .map(|(_, ty)| ty)
                                    .unique()
                                    .map(|ty| format!(
                                        "`{}`",
                                        ty.display(self.view, &disambiguation)
                                    ))
                                    .join(" and ")
                            ),
                            instr.source,
                        )
                        .into());
                    };

                    let mut a_cast = None;
                    let mut b_cast = None;

                    for (index, cast) in unified.unary_casts {
                        match index {
                            false => a_cast = Some(cast),
                            true => b_cast = Some(cast),
                        }
                    }

                    cfg.set_binop_unary_casts(instr_ref, a_cast, b_cast);
                    cfg.set_typed(instr_ref, unified.unified);
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
                    let name_path = call.name_path;
                    let basename = name_path.basename();

                    let view = if name_path.has_namespace() {
                        let resolved_namespace = self
                            .resolved_namespace
                            .get_or_insert_with(|| ResolveNamespace::new(self.view, instr.source));

                        execute_sub_task!(
                            self,
                            resolved_namespace,
                            executor,
                            ctx,
                            name_path.namespaces()
                        )
                    } else {
                        *self.view
                    };

                    let symbol = match view.find_symbol(
                        executor,
                        FuncSearch {
                            name: basename,
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
                        let got_type = cfg.get_typed(*arg, builtin_types);

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
                                let disambiguation = TypeDisplayerDisambiguation::new(
                                    [expected_type.0, got_type.0].into_iter(),
                                );

                                return Err(ErrorDiagnostic::new(
                                    format!(
                                        "Expected `{}` for argument #{}, but got `{}`",
                                        expected_type.display(self.view, &disambiguation),
                                        i + 1,
                                        got_type.display(self.view, &disambiguation)
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

                    cfg.set_typed_and_callee(
                        instr_ref,
                        func_head,
                        arg_casts,
                        variadic_arg_types,
                        func_head.view,
                    );
                }
                InstrKind::DeclareAssign(_, value, _, _) => {
                    // 1] Conform the value to its default concrete type
                    let conformed = conform_to_default(
                        ctx,
                        cfg.get_typed(*value, builtin_types),
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
                InstrKind::StructLiteral(struct_literal, _) => {
                    // Resolve type head
                    let Some(resolved_type) = executor.demand(self.resolved_type) else {
                        return suspend!(
                            self.resolved_type,
                            executor.request(ResolveType::new(self.view, struct_literal.ast_type)),
                            ctx
                        );
                    };

                    // Extract struct type
                    let TypeKind::UserDefined(UserDefinedType {
                        name: _,
                        rest:
                            TypeHeadRest {
                                kind: TypeHeadRestKind::Struct(structure),
                                view: struct_view,
                            },
                        args: _,
                    }) = &resolved_type.0.kind
                    else {
                        return Err(ErrorDiagnostic::new(
                            "Cannot create struct literal for non-struct type",
                            instr.source,
                        )
                        .into());
                    };

                    // Resolve body of struct type
                    let Some(resolved_struct_body) = executor.demand(self.resolved_struct_body)
                    else {
                        return suspend!(
                            self.resolved_struct_body,
                            executor.request(ResolveStructureBody::new(
                                struct_view,
                                &self.compiler,
                                structure
                            )),
                            ctx
                        );
                    };

                    // Resolve field initializers (no suspend)
                    let unary_casts_and_indices = (|| -> Result<_, ErrorDiagnostic> {
                        // Conform field values to field types
                        let mut unary_casts_and_indices =
                            Vec::with_capacity(struct_literal.fields.len());
                        let mut next_index = 0;
                        let mut seen = HashSet::new();

                        for field_init in struct_literal.fields {
                            let Some(field_name) = field_init.name.or_else(|| {
                                structure
                                    .fields
                                    .get_index(next_index)
                                    .map(|(n, _)| n.as_str())
                            }) else {
                                return Err(ErrorDiagnostic::new(
                                    "Out of fields to populate for struct literal",
                                    instr.source,
                                )
                                .into());
                            };

                            if !seen.insert(field_name) {
                                return Err(ErrorDiagnostic::new(
                                    format!(
                                        "Field `{}` cannot be specified more than once",
                                        field_name
                                    ),
                                    instr.source,
                                )
                                .into());
                            }

                            let Some((index, _, field)) =
                                resolved_struct_body.fields.get_full(field_name)
                            else {
                                return Err(ErrorDiagnostic::new(
                                    format!("Field `{}` does not exist on struct", field_name),
                                    instr.source,
                                )
                                .into());
                            };

                            let conform_behavior = struct_literal.conform_behavior;

                            let mode = match conform_behavior {
                                ConformBehavior::Adept(_) => ConformMode::Normal,
                                ConformBehavior::C => ConformMode::Explicit,
                            };

                            let got_type = cfg.get_typed(field_init.value, builtin_types);
                            let Some(conformed) = conform_to(
                                ctx,
                                got_type,
                                field.ty,
                                conform_behavior.c_integer_assumptions(),
                                builtin_types,
                                self.view.target(),
                                mode,
                                |_, _| {
                                    unimplemented!(
                                        "on polymorph not supported for polymorphic struct type literals yet"
                                    )
                                },
                                instr.source,
                            ) else {
                                let expected_type = field.ty.0;
                                let disambiguation = TypeDisplayerDisambiguation::new(
                                    [expected_type, got_type.0].into_iter(),
                                );

                                return Err(ErrorDiagnostic::new(
                                    format!(
                                        "Expected value of type `{}` for field `{}`",
                                        field.ty.display(struct_view, &disambiguation),
                                        field_name
                                    ),
                                    instr.source,
                                )
                                .into());
                            };

                            unary_casts_and_indices.push((index, conformed.cast));
                            next_index = index + 1;
                        }

                        // TODO: Fill in remaining fields according to fill behavior
                        if !struct_literal.fill_behavior.is_forbid() {
                            return Err(ErrorDiagnostic::new(
                                format!(
                                    "Unimplemented struct literal fill behavior - {:?}",
                                    struct_literal.fill_behavior
                                ),
                                instr.source,
                            )
                            .into());
                        }

                        // Report error for any missing fields
                        if struct_literal.fields.len() != structure.fields.len() {
                            let missing_fields = structure
                                .fields
                                .keys()
                                .filter(|name| !seen.contains(name.as_str()))
                                .map(|name| format!("`{}`", name))
                                .collect_vec();

                            let message = if missing_fields.len() == 1 {
                                format!("Missing field {} for struct literal", missing_fields[0])
                            } else if missing_fields.len() <= 5 {
                                let before_and = missing_fields
                                    .iter()
                                    .take(missing_fields.len() - 1)
                                    .join(", ");

                                let after_and = &missing_fields[missing_fields.len() - 1];

                                format!(
                                    "Missing fields {}, and {} for struct literal",
                                    before_and, after_and
                                )
                            } else {
                                format!(
                                    "Missing fields {}, and more, for struct literal",
                                    missing_fields.iter().take(5).join(", ")
                                )
                            };

                            return Err(ErrorDiagnostic::new(message, instr.source).into());
                        }

                        Ok(unary_casts_and_indices)
                    })()?;

                    // WARNING: We should never suspend anymore here, due to previous no suspend
                    // block...

                    cfg.set_struct_literal_unary_casts_and_indices(
                        instr_ref,
                        ctx.alloc_slice_fill_iter(unary_casts_and_indices.into_iter()),
                    );
                    cfg.set_typed(instr_ref, resolved_type);
                }
                InstrKind::UnaryOperation(unary_operator, value, _) => {
                    match unary_operator {
                        ast::UnaryOperator::Math(ast::UnaryMathOperator::Not) => {
                            let value_ty = cfg.get_typed(*value, builtin_types);

                            if let TypeKind::BooleanLiteral(bool_value) = &value_ty.0.kind {
                                cfg.set_typed(
                                    instr_ref,
                                    UnaliasedType(ctx.alloc(
                                        TypeKind::BooleanLiteral(!bool_value).at(instr.source),
                                    )),
                                );
                            } else {
                                // 1] Conform the value to its default concrete type
                                let conformed = conform_to_default(
                                    ctx,
                                    value_ty,
                                    c_integer_assumptions,
                                    builtin_types,
                                    self.view.target(),
                                )?;

                                if !conformed.ty.0.kind.is_boolean() {
                                    return Err(ErrorDiagnostic::new(
                                        format!(
                                            "Cannot perform not operator on non-bool value of type `{}`",
                                            conformed.ty.display_one(self.view)
                                        ),
                                        instr.source,
                                    )
                                    .into());
                                }

                                cfg.set_primary_unary_cast(instr_ref, conformed.cast);
                                cfg.set_typed(instr_ref, builtin_types.bool());
                            }
                        }
                        ast::UnaryOperator::Math(ast::UnaryMathOperator::Negate) => todo!(),
                        ast::UnaryOperator::Math(ast::UnaryMathOperator::IsNonZero) => todo!(),
                        ast::UnaryOperator::Math(ast::UnaryMathOperator::BitComplement) => todo!(),
                        ast::UnaryOperator::AddressOf => todo!(),
                        ast::UnaryOperator::Dereference => {
                            let conformed = conform_to_default(
                                ctx,
                                cfg.get_typed(*value, builtin_types),
                                c_integer_assumptions,
                                builtin_types,
                                self.view.target(),
                            )?;

                            if !conformed.ty.0.kind.is_ptr() {
                                return Err(ErrorDiagnostic::new(
                                    format!(
                                        "Cannot dereference non-pointer value of type `{}`",
                                        conformed.ty.display_one(self.view)
                                    ),
                                    instr.source,
                                )
                                .into());
                            }

                            cfg.set_primary_unary_cast(instr_ref, conformed.cast);
                            cfg.set_typed(instr_ref, builtin_types.bool());
                        }
                    }
                }
                InstrKind::SizeOf(_, size_of_mode) => todo!("sizeof"),
                InstrKind::SizeOfValue(instr_ref, size_of_mode) => todo!("sizeof value"),
                InstrKind::InterpreterSyscall(interpreter_syscall_instr) => {
                    todo!("interpreter syscall")
                }
                InstrKind::IntegerPromote(instr_ref) => todo!("integer promote"),
                InstrKind::ConformToBool(value, language, _) => {
                    let conformed = conform_to_default(
                        ctx,
                        cfg.get_typed(*value, builtin_types),
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

            // Reset suspension states for next instruction to use
            self.resolved_type = None;
            self.resolved_namespace = None;
            self.resolved_struct_body = None;

            rev_post_order.next_in_builder(cfg, post_order);
        }

        let final_cfg = ctx.alloc(self.cfg.take().unwrap().finish(ctx));

        Ok(ctx.alloc(FuncBody {
            cfg: final_cfg,
            post_order: ctx.alloc_slice_fill_iter(std::mem::take(post_order).iter().copied()),
            variables: std::mem::take(variables),
        }))
    }
}
