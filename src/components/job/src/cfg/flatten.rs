use crate::{
    CfgValue, ExecutionCtx, Label,
    cfg::{
        IsValue,
        builder::{CfgBuilder, Cursor},
        instr::{
            BreakContinue, CallInstr, EndInstrKind, FieldInitializer, InstrKind,
            InterpreterSyscallInstr, StructLiteralInstr,
        },
    },
};
use ast::{ConformBehavior, Language, Params, Stmt};
use source_files::Source;

pub fn flatten_func<'env>(
    ctx: &mut ExecutionCtx<'env>,
    params: &'env Params,
    stmts: &'env [Stmt],
    source: Source,
) -> CfgBuilder<'env> {
    let (mut builder, mut cursor) = CfgBuilder::<'env>::new();

    for (index, param) in params.required.iter().enumerate() {
        let Some(name) = &param.name else {
            continue;
        };

        builder.try_push(
            &mut cursor,
            InstrKind::Parameter(
                name,
                &param.ast_type,
                index.try_into().expect("reasonable number of parameters"),
                None,
            )
            .at(source),
        );
    }

    flatten_stmts(ctx, &mut builder, &mut cursor, stmts, IsValue::NeglectValue);
    builder.try_push_end(
        EndInstrKind::Return(CfgValue::Void, None).at(source),
        &mut cursor,
    );
    builder
}

fn flatten_stmts<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    stmts: &'env [Stmt],
    is_value: IsValue,
) -> CfgValue {
    let mut out_value = CfgValue::Void;

    let length = stmts.len();
    for (i, stmt) in stmts.iter().enumerate() {
        out_value = flatten_stmt(
            ctx,
            builder,
            cursor,
            stmt,
            if i + 1 == length {
                is_value
            } else {
                IsValue::NeglectValue
            },
        );
    }

    out_value
}

fn flatten_stmt<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    stmt: &'env Stmt,
    is_value: IsValue,
) -> CfgValue {
    match &stmt.kind {
        ast::StmtKind::Return(expr) => {
            let value = expr
                .as_ref()
                .map(|expr| flatten_expr(ctx, builder, cursor, expr, IsValue::RequireValue));

            builder
                .try_push_end(
                    EndInstrKind::Return(
                        value.unwrap_or_else(|| builder.never_or_void(cursor)),
                        None,
                    )
                    .at(stmt.source),
                    cursor,
                )
                .into()
        }
        ast::StmtKind::Expr(expr) => flatten_expr(ctx, builder, cursor, expr, is_value),
        ast::StmtKind::Declaration(declaration) => {
            let initial_value = declaration.initial_value.as_ref().map(|initial_value| {
                flatten_expr(ctx, builder, cursor, initial_value, IsValue::RequireValue)
            });

            builder
                .try_push(
                    cursor,
                    InstrKind::Declare(
                        &declaration.name,
                        &declaration.ast_type,
                        initial_value,
                        None,
                        None,
                    )
                    .at(stmt.source),
                )
                .into()
        }
        ast::StmtKind::Assignment(assignment) => {
            let raw_dest = flatten_expr(
                ctx,
                builder,
                cursor,
                &assignment.destination,
                IsValue::RequireValue,
            );

            let dest = CfgValue::Instr(
                builder.try_push(cursor, InstrKind::IntoDest(raw_dest, None).at(stmt.source)),
            );

            let raw_src = flatten_expr(
                ctx,
                builder,
                cursor,
                &assignment.value,
                IsValue::RequireValue,
            );

            let src = if let Some(operator) = assignment.operator {
                CfgValue::Instr(
                    builder.try_push(
                        cursor,
                        InstrKind::BinOp(
                            dest,
                            operator,
                            raw_src,
                            assignment.conform_behavior,
                            None,
                            None,
                            None,
                        )
                        .at(stmt.source),
                    ),
                )
            } else {
                raw_src
            };

            builder.try_push(
                cursor,
                InstrKind::Assign {
                    dest,
                    src,
                    src_cast: None,
                }
                .at(stmt.source),
            );

            builder.never_or_void(cursor)
        }
        ast::StmtKind::Label(name) => {
            builder.push_jump_to_new_block(cursor, stmt.source);

            builder.add_label(Label::new(
                name,
                cursor.basicblock().into_raw(),
                stmt.source,
            ));

            builder.never_or_void(cursor)
        }
        ast::StmtKind::Goto(label_name) => builder
            .try_push_end(
                EndInstrKind::IncompleteGoto(label_name).at(stmt.source),
                cursor,
            )
            .into(),
    }
}

fn flatten_expr<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    expr: &'env ast::Expr,
    is_value: IsValue,
) -> CfgValue {
    match &expr.kind {
        ast::ExprKind::Variable(name) => {
            let name = name
                .as_plain_str()
                .expect("only plain names can be used with new cfg");

            builder
                .try_push(cursor, InstrKind::Name(name, None).at(expr.source))
                .into()
        }
        ast::ExprKind::Boolean(value) => builder
            .try_push(cursor, InstrKind::BooleanLiteral(*value).at(expr.source))
            .into(),
        ast::ExprKind::Integer(integer) => builder
            .try_push(cursor, InstrKind::IntegerLiteral(integer).at(expr.source))
            .into(),
        ast::ExprKind::Float(float) => builder
            .try_push(cursor, InstrKind::FloatLiteral(*float).at(expr.source))
            .into(),
        ast::ExprKind::Char(char) => builder
            .try_push(cursor, InstrKind::Utf8CharLiteral(char).at(expr.source))
            .into(),
        ast::ExprKind::String(string) => builder
            .try_push(cursor, InstrKind::StringLiteral(string).at(expr.source))
            .into(),
        ast::ExprKind::NullTerminatedString(cstring) => builder
            .try_push(
                cursor,
                InstrKind::NullTerminatedStringLiteral(cstring).at(expr.source),
            )
            .into(),
        ast::ExprKind::CharLiteral(ascii_char) => builder
            .try_push(
                cursor,
                InstrKind::AsciiCharLiteral(*ascii_char).at(expr.source),
            )
            .into(),
        ast::ExprKind::Null => builder
            .try_push(cursor, InstrKind::NullLiteral.at(expr.source))
            .into(),
        ast::ExprKind::Call(call) => {
            let args = ctx.alloc_slice_fill_iter(
                call.args
                    .iter()
                    .map(|arg| flatten_expr(ctx, builder, cursor, arg, IsValue::RequireValue))
                    .into_iter(),
            );

            let generics = ctx.alloc_slice_fill_iter(call.generics.iter().map(|generic_arg| {
                match generic_arg {
                    ast::TypeArg::Type(ty) => ty,
                    ast::TypeArg::Expr(_) => todo!("expressions not supported for generics for function calls in new system yet"),
                }
            }));

            builder
                .try_push(
                    cursor,
                    InstrKind::Call(
                        ctx.alloc(CallInstr {
                            name_path: &call.name_path,
                            args,
                            expected_to_return: call.expected_to_return.as_ref(),
                            generics,
                        }),
                        None,
                    )
                    .at(expr.source),
                )
                .into()
        }
        ast::ExprKind::DeclareAssign(declare_assign) => {
            let value = flatten_expr(
                ctx,
                builder,
                cursor,
                &declare_assign.value,
                IsValue::RequireValue,
            );

            builder
                .try_push(
                    cursor,
                    InstrKind::DeclareAssign(&declare_assign.name, value, None, None)
                        .at(expr.source),
                )
                .into()
        }
        ast::ExprKind::BasicBinaryOperation(bin_op) => {
            let left = flatten_expr(ctx, builder, cursor, &bin_op.left, IsValue::RequireValue);
            let right = flatten_expr(ctx, builder, cursor, &bin_op.right, IsValue::RequireValue);

            builder
                .try_push(
                    cursor,
                    InstrKind::BinOp(
                        left,
                        bin_op.operator,
                        right,
                        ConformBehavior::C,
                        None,
                        None,
                        None,
                    )
                    .at(expr.source),
                )
                .into()
        }
        ast::ExprKind::ShortCircuitingBinaryOperation(bin_op) => {
            let right_source = bin_op.right.source;

            let (mut when_true, mut when_false) = flatten_condition(
                ctx,
                builder,
                cursor,
                &bin_op.left,
                bin_op.conform_behavior.language(),
                Some(bin_op.conform_behavior),
            );

            let when_more_calc = match bin_op.operator {
                ast::ShortCircuitingBinaryOperator::And => when_true.cursor(),
                ast::ShortCircuitingBinaryOperator::Or => when_false.cursor(),
            };

            // NOTE: For C, the pre-conforming value should be the result, but we don't do that yet
            let right_unconformed = flatten_expr(
                ctx,
                builder,
                when_more_calc,
                &bin_op.right,
                IsValue::RequireValue,
            );

            builder.try_push(
                when_more_calc,
                InstrKind::ConformToBool(
                    right_unconformed,
                    bin_op.conform_behavior.language(),
                    None,
                )
                .at(right_source),
            );

            new_basicblock_joining(
                ctx,
                builder,
                [when_true, when_false].into_iter(),
                Some(bin_op.conform_behavior),
                expr.source,
            )
            .value
        }
        ast::ExprKind::Member(subject, member, privacy) => {
            let subject = flatten_expr(ctx, builder, cursor, subject, IsValue::RequireValue);

            builder
                .try_push(
                    cursor,
                    InstrKind::Member(subject, member, *privacy).at(expr.source),
                )
                .into()
        }
        ast::ExprKind::ArrayAccess(array_access) => {
            let subject = flatten_expr(
                ctx,
                builder,
                cursor,
                &array_access.subject,
                IsValue::RequireValue,
            );

            let index = flatten_expr(
                ctx,
                builder,
                cursor,
                &array_access.index,
                IsValue::RequireValue,
            );

            builder
                .try_push(
                    cursor,
                    InstrKind::ArrayAccess(subject, index).at(expr.source),
                )
                .into()
        }
        ast::ExprKind::StructLiteral(struct_literal) => {
            let fields =
                ctx.alloc_slice_fill_iter(struct_literal.fields.iter().map(|x| FieldInitializer {
                    name: x.name.as_ref().map(|name| name.as_str()),
                    value: flatten_expr(ctx, builder, cursor, &x.value, IsValue::RequireValue),
                }));

            let literal = ctx.alloc(StructLiteralInstr {
                ast_type: &struct_literal.ast_type,
                fields,
                fill_behavior: struct_literal.fill_behavior,
                conform_behavior: struct_literal.conform_behavior,
            });

            builder
                .try_push(
                    cursor,
                    InstrKind::StructLiteral(literal, None).at(expr.source),
                )
                .into()
        }
        ast::ExprKind::UnaryOperation(unary_op) => {
            let inner = flatten_expr(ctx, builder, cursor, &unary_op.inner, IsValue::RequireValue);

            builder
                .try_push(
                    cursor,
                    InstrKind::UnaryOperation(unary_op.operator, inner, None).at(expr.source),
                )
                .into()
        }
        ast::ExprKind::Conditional(conditional) => {
            let (in_scope, close_scope) = builder.push_scope(cursor, expr.source);
            *cursor = in_scope;

            let mut incoming = vec![];

            for (condition, block) in conditional.conditions.iter() {
                let (mut when_true, when_false) = flatten_condition(
                    ctx,
                    builder,
                    cursor,
                    condition,
                    conditional.conform_behavior.language(),
                    Some(conditional.conform_behavior),
                );

                let when_true_value =
                    flatten_stmts(ctx, builder, when_true.cursor(), &block.stmts, is_value);

                incoming.push(JoinedCursor::new(
                    match is_value {
                        IsValue::RequireValue => when_true_value,
                        IsValue::NeglectValue => builder.never_or_void(cursor),
                    },
                    when_true.cursor,
                ));
                *cursor = when_false.cursor;
            }

            if let Some(otherwise) = &conditional.otherwise {
                let inner = flatten_stmts(ctx, builder, cursor, &otherwise.stmts, is_value);

                incoming.push(JoinedCursor::new(
                    match is_value {
                        IsValue::RequireValue => inner,
                        IsValue::NeglectValue => builder.never_or_void(cursor),
                    },
                    std::mem::replace(cursor, close_scope),
                ));
            } else {
                incoming.push(JoinedCursor::new(
                    builder.never_or_void(cursor),
                    std::mem::replace(cursor, close_scope),
                ));
            }

            existing_basicblock_joining(
                ctx,
                builder,
                cursor,
                incoming.into_iter(),
                Some(conditional.conform_behavior),
                expr.source,
            )
        }
        ast::ExprKind::While(while_loop) => {
            builder.push_jump_to_new_block(cursor, expr.source);

            let condition_bb = cursor.basicblock().into_raw();
            let condition = flatten_expr(
                ctx,
                builder,
                cursor,
                &while_loop.condition,
                IsValue::RequireValue,
            );

            let (mut when_true, when_false) = builder.try_push_branch(
                condition,
                cursor,
                Some(BreakContinue::positive()),
                expr.source,
            );

            flatten_stmts(
                ctx,
                builder,
                &mut when_true,
                &while_loop.block.stmts,
                IsValue::NeglectValue,
            );

            builder.try_push_end(
                EndInstrKind::Jump(
                    condition_bb,
                    builder.never_or_void(&mut when_true),
                    None,
                    None,
                )
                .at(expr.source),
                &mut when_true,
            );

            *cursor = when_false;
            builder.never_or_void(cursor)
        }
        ast::ExprKind::StaticMemberValue(_) => todo!(),
        ast::ExprKind::StaticMemberCall(_) => todo!(),
        ast::ExprKind::SizeOf(ty, mode) => builder
            .try_push(cursor, InstrKind::SizeOf(ty, *mode).at(expr.source))
            .into(),
        ast::ExprKind::SizeOfValue(of_value, mode) => {
            let value = flatten_expr(ctx, builder, cursor, of_value, IsValue::RequireValue);
            builder
                .try_push(cursor, InstrKind::SizeOfValue(value, *mode).at(expr.source))
                .into()
        }
        ast::ExprKind::InterpreterSyscall(syscall) => {
            let args = ctx.alloc_slice_fill_iter(syscall.args.iter().map(|(ty, arg)| {
                (
                    ty,
                    flatten_expr(ctx, builder, cursor, arg, IsValue::RequireValue),
                )
            }));

            builder
                .try_push(
                    cursor,
                    InstrKind::InterpreterSyscall(ctx.alloc(InterpreterSyscallInstr {
                        kind: syscall.kind,
                        args,
                        result_type: &syscall.result_type,
                    }))
                    .at(expr.source),
                )
                .into()
        }
        ast::ExprKind::Break => builder
            .try_push_end(EndInstrKind::IncompleteBreak.at(expr.source), cursor)
            .into(),
        ast::ExprKind::Continue => builder
            .try_push_end(EndInstrKind::IncompleteContinue.at(expr.source), cursor)
            .into(),
        ast::ExprKind::IntegerPromote(value) => {
            let value = flatten_expr(ctx, builder, cursor, value, IsValue::RequireValue);
            builder
                .try_push(cursor, InstrKind::IntegerPromote(value).at(expr.source))
                .into()
        }
        ast::ExprKind::StaticAssert(..) => todo!(),
        ast::ExprKind::Is(value, variant) => {
            let value = flatten_expr(ctx, builder, cursor, &value, IsValue::RequireValue);
            builder
                .try_push(cursor, InstrKind::Is(value, &variant).at(expr.source))
                .into()
        }
        ast::ExprKind::LabelLiteral(label_name) => builder
            .try_push(cursor, InstrKind::LabelLiteral(label_name).at(expr.source))
            .into(),
        ast::ExprKind::Comptime(comptime_expr) => {
            let (mut comptime_builder, mut comptime_cursor) = CfgBuilder::new();

            let cfg_value = flatten_expr(
                ctx,
                &mut comptime_builder,
                &mut comptime_cursor,
                comptime_expr,
                IsValue::RequireValue,
            );

            builder
                .try_push(
                    cursor,
                    InstrKind::Comptime(comptime_builder).at(expr.source),
                )
                .into()
        }
    }
}

fn flatten_condition<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    expr: &'env ast::Expr,
    language: Language,
    conform_behavior: Option<ConformBehavior>,
) -> (JoinedCursor, JoinedCursor) {
    let source = expr.source;

    let mut when_true = vec![];
    let mut when_false = vec![];
    flatten_condition_inner(
        ctx,
        builder,
        cursor,
        expr,
        language,
        &mut when_true,
        &mut when_false,
    );

    let when_true = new_basicblock_joining(
        ctx,
        builder,
        when_true.into_iter(),
        conform_behavior,
        source,
    );
    let when_false = new_basicblock_joining(
        ctx,
        builder,
        when_false.into_iter(),
        conform_behavior,
        source,
    );
    (when_true, when_false)
}

struct JoinedCursor {
    value: CfgValue,
    cursor: Cursor,
}

impl JoinedCursor {
    pub fn new(value: CfgValue, cursor: Cursor) -> Self {
        Self { value, cursor }
    }

    pub fn cursor(&mut self) -> &mut Cursor {
        &mut self.cursor
    }
}

/// Creates a new basicblock that joins multiple basicblocks together using
/// jumps and a PHI node.
fn new_basicblock_joining<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    mut incoming: impl ExactSizeIterator<Item = JoinedCursor>,
    conform_behavior: Option<ConformBehavior>,
    source: Source,
) -> JoinedCursor {
    if incoming.len() == 1 {
        return incoming.next().unwrap();
    }

    let mut joined_bb = builder.new_block();

    JoinedCursor {
        value: existing_basicblock_joining(
            ctx,
            builder,
            &mut joined_bb,
            incoming,
            conform_behavior,
            source,
        ),
        cursor: joined_bb,
    }
}

fn existing_basicblock_joining<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    to: &mut Cursor,
    incoming: impl IntoIterator<Item = JoinedCursor>,
    conform_behavior: Option<ConformBehavior>,
    source: Source,
) -> CfgValue {
    let mut values = Vec::new();

    for mut from in incoming {
        if builder.has_end(&from.cursor) {
            continue;
        }

        let value = from.value;
        values.push((from.cursor().basicblock().into_raw(), value));

        builder.try_push_end(
            EndInstrKind::Jump(to.basicblock().into_raw(), value, None, None).at(source),
            from.cursor(),
        );
    }

    builder
        .try_push(
            to,
            InstrKind::Phi {
                possible_incoming: ctx.alloc_slice_fill_iter(values.into_iter()),
                conform_behavior,
            }
            .at(source),
        )
        .into()
}

fn flatten_condition_inner<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    expr: &'env ast::Expr,
    language: Language,
    join_when_true: &mut Vec<JoinedCursor>,
    join_when_false: &mut Vec<JoinedCursor>,
) {
    let source = expr.source;

    match &expr.kind {
        ast::ExprKind::ShortCircuitingBinaryOperation(bin_op) => {
            let mut more_to_compute = Vec::new();

            match bin_op.operator {
                ast::ShortCircuitingBinaryOperator::And => flatten_condition_inner(
                    ctx,
                    builder,
                    cursor,
                    &bin_op.left,
                    bin_op.conform_behavior.language(),
                    &mut more_to_compute,
                    join_when_false,
                ),
                ast::ShortCircuitingBinaryOperator::Or => flatten_condition_inner(
                    ctx,
                    builder,
                    cursor,
                    &bin_op.left,
                    bin_op.conform_behavior.language(),
                    join_when_true,
                    &mut more_to_compute,
                ),
            }
        }
        _ => {
            let value = flatten_expr(ctx, builder, cursor, expr, IsValue::RequireValue);
            let condition = builder.try_push(
                cursor,
                InstrKind::ConformToBool(value, language, None).at(source),
            );

            let (when_true, when_false) =
                builder.try_push_branch(condition.into(), cursor, None, source);

            join_when_true.push(JoinedCursor::new(
                builder.never_or_void(&when_true),
                when_true,
            ));

            join_when_false.push(JoinedCursor::new(
                builder.never_or_void(&when_false),
                when_false,
            ));
        }
    }
}
