use crate::{
    ExecutionCtx, InstrRef, Label,
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

        builder.push(
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

    builder.push_end(&mut cursor, EndInstrKind::Return(None, None).at(source));
    builder
}

fn flatten_stmts<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    stmts: &'env [Stmt],
    is_value: IsValue,
) -> Option<InstrRef> {
    let mut value = None;

    let length = stmts.len();
    for (i, stmt) in stmts.iter().enumerate() {
        value = flatten_stmt(
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

    value
}

fn flatten_stmt<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    stmt: &'env Stmt,
    is_value: IsValue,
) -> Option<InstrRef> {
    match &stmt.kind {
        ast::StmtKind::Return(expr) => {
            let value = expr
                .as_ref()
                .map(|expr| flatten_expr(ctx, builder, cursor, expr, IsValue::RequireValue));
            builder.push_end(cursor, EndInstrKind::Return(value, None).at(stmt.source));
            None
        }
        ast::StmtKind::Expr(expr) => Some(flatten_expr(ctx, builder, cursor, expr, is_value)),
        ast::StmtKind::Declaration(declaration) => {
            let initial_value = declaration.initial_value.as_ref().map(|initial_value| {
                flatten_expr(ctx, builder, cursor, initial_value, IsValue::RequireValue)
            });

            Some(
                builder.push(
                    cursor,
                    InstrKind::Declare(
                        &declaration.name,
                        &declaration.ast_type,
                        initial_value,
                        None,
                        None,
                    )
                    .at(stmt.source),
                ),
            )
        }
        ast::StmtKind::Assignment(assignment) => {
            let left = flatten_expr(
                ctx,
                builder,
                cursor,
                &assignment.value,
                IsValue::RequireValue,
            );

            let right = flatten_expr(
                ctx,
                builder,
                cursor,
                &assignment.destination,
                IsValue::RequireValue,
            );

            builder.push(cursor, InstrKind::Assign(left, right).at(stmt.source));
            None
        }
        ast::StmtKind::Label(name) => {
            builder.push_jump_to_new_block(cursor, stmt.source);

            builder.add_label(Label::new(
                name,
                cursor.basicblock().into_raw(),
                stmt.source,
            ));
            None
        }
        ast::StmtKind::Goto(label_name) => {
            builder.push_end(
                cursor,
                EndInstrKind::IncompleteGoto(label_name).at(stmt.source),
            );
            None
        }
    }
}

fn flatten_expr<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    expr: &'env ast::Expr,
    is_value: IsValue,
) -> InstrRef {
    match &expr.kind {
        ast::ExprKind::Variable(name) => {
            let name = name
                .as_plain_str()
                .expect("only plain names can be used with new cfg");
            builder.push(cursor, InstrKind::Name(name, None).at(expr.source))
        }
        ast::ExprKind::Boolean(value) => {
            builder.push(cursor, InstrKind::BooleanLiteral(*value).at(expr.source))
        }
        ast::ExprKind::Integer(integer) => {
            builder.push(cursor, InstrKind::IntegerLiteral(integer).at(expr.source))
        }
        ast::ExprKind::Float(float) => {
            builder.push(cursor, InstrKind::FloatLiteral(*float).at(expr.source))
        }
        ast::ExprKind::Char(char) => {
            builder.push(cursor, InstrKind::Utf8CharLiteral(char).at(expr.source))
        }
        ast::ExprKind::String(string) => {
            builder.push(cursor, InstrKind::StringLiteral(string).at(expr.source))
        }
        ast::ExprKind::NullTerminatedString(cstring) => builder.push(
            cursor,
            InstrKind::NullTerminatedStringLiteral(cstring).at(expr.source),
        ),
        ast::ExprKind::CharLiteral(ascii_char) => builder.push(
            cursor,
            InstrKind::AsciiCharLiteral(*ascii_char).at(expr.source),
        ),
        ast::ExprKind::Null => builder.push(cursor, InstrKind::NullLiteral.at(expr.source)),
        ast::ExprKind::Call(call) => {
            let name = call
                .name
                .as_plain_str()
                .expect("only plain names can be used for new cfg system");

            let args = ctx.alloc_slice_fill_iter(
                call.args
                    .iter()
                    .map(|arg| flatten_expr(ctx, builder, cursor, arg, IsValue::RequireValue)),
            );

            let generics = ctx.alloc_slice_fill_iter(call.generics.iter().map(|generic_arg| {
                match generic_arg {
                    ast::TypeArg::Type(ty) => ty,
                    ast::TypeArg::Expr(_) => todo!("expressions not supported for generics for function calls in new system yet"),
                }
            }));

            builder.push(
                cursor,
                InstrKind::Call(
                    ctx.alloc(CallInstr {
                        name,
                        args,
                        expected_to_return: call.expected_to_return.as_ref(),
                        generics,
                    }),
                    None,
                )
                .at(expr.source),
            )
        }
        ast::ExprKind::DeclareAssign(declare_assign) => {
            let value = flatten_expr(
                ctx,
                builder,
                cursor,
                &declare_assign.value,
                IsValue::RequireValue,
            );
            builder.push(
                cursor,
                InstrKind::DeclareAssign(&declare_assign.name, value, None, None).at(expr.source),
            )
        }
        ast::ExprKind::BasicBinaryOperation(bin_op) => {
            let left = flatten_expr(ctx, builder, cursor, &bin_op.left, IsValue::RequireValue);
            let right = flatten_expr(ctx, builder, cursor, &bin_op.right, IsValue::RequireValue);

            builder.push(
                cursor,
                InstrKind::BinOp(left, bin_op.operator, right, bin_op.language).at(expr.source),
            )
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

            builder.push(
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
            .instr
            .unwrap_or_else(|| builder.push(cursor, InstrKind::VoidLiteral.at(expr.source)))
        }
        ast::ExprKind::Member(subject, member, privacy) => {
            let subject = flatten_expr(ctx, builder, cursor, subject, IsValue::RequireValue);

            builder.push(
                cursor,
                InstrKind::Member(subject, member, *privacy).at(expr.source),
            )
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

            builder.push(
                cursor,
                InstrKind::ArrayAccess(subject, index).at(expr.source),
            )
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
                language: struct_literal.language,
            });

            builder.push(cursor, InstrKind::StructLiteral(literal).at(expr.source))
        }
        ast::ExprKind::UnaryOperation(unary_op) => {
            let inner = flatten_expr(ctx, builder, cursor, &unary_op.inner, IsValue::RequireValue);

            builder.push(
                cursor,
                InstrKind::UnaryOperation(unary_op.operator, inner).at(expr.source),
            )
        }
        ast::ExprKind::Conditional(conditional) => {
            let (in_scope, close_scope) = builder.push_scope(cursor, expr.source);
            *cursor = in_scope;

            let mut incoming = vec![];

            let no_result = match is_value {
                IsValue::RequireValue => None,
                IsValue::NeglectValue => {
                    Some(builder.push(cursor, InstrKind::VoidLiteral.at(expr.source)))
                }
            };

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
                let value = no_result.or(when_true_value);

                incoming.push(Joined::new(when_true.cursor, value));
                *cursor = when_false.cursor;
            }

            if let Some(otherwise) = &conditional.otherwise {
                let value = flatten_stmts(ctx, builder, cursor, &otherwise.stmts, is_value);
                incoming.push(Joined::new(std::mem::replace(cursor, close_scope), value));
            } else {
                builder.push(cursor, InstrKind::VoidLiteral.at(expr.source));
                incoming.push(Joined::new(
                    std::mem::replace(cursor, close_scope),
                    no_result,
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
            let void = builder.push(cursor, InstrKind::VoidLiteral.at(expr.source));

            builder.push_jump_to_new_block(cursor, expr.source);

            let condition_bb = cursor.basicblock().into_raw();
            let condition = flatten_expr(
                ctx,
                builder,
                cursor,
                &while_loop.condition,
                IsValue::RequireValue,
            );

            let (mut when_true, when_false) = builder.push_branch(
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

            builder.push_end(
                &mut when_true,
                EndInstrKind::Jump(condition_bb, None, None).at(expr.source),
            );

            *cursor = when_false;
            void
        }
        ast::ExprKind::StaticMemberValue(_) => todo!(),
        ast::ExprKind::StaticMemberCall(_) => todo!(),
        ast::ExprKind::SizeOf(ty, mode) => {
            builder.push(cursor, InstrKind::SizeOf(ty, *mode).at(expr.source))
        }
        ast::ExprKind::SizeOfValue(of_value, mode) => {
            let value = flatten_expr(ctx, builder, cursor, of_value, IsValue::RequireValue);
            builder.push(cursor, InstrKind::SizeOfValue(value, *mode).at(expr.source))
        }
        ast::ExprKind::InterpreterSyscall(syscall) => {
            let args = ctx.alloc_slice_fill_iter(syscall.args.iter().map(|(ty, arg)| {
                (
                    ty,
                    flatten_expr(ctx, builder, cursor, arg, IsValue::RequireValue),
                )
            }));

            builder.push(
                cursor,
                InstrKind::InterpreterSyscall(ctx.alloc(InterpreterSyscallInstr {
                    kind: syscall.kind,
                    args,
                    result_type: &syscall.result_type,
                }))
                .at(expr.source),
            )
        }
        ast::ExprKind::Break => {
            builder.push_end(cursor, EndInstrKind::IncompleteBreak.at(expr.source))
        }
        ast::ExprKind::Continue => {
            builder.push_end(cursor, EndInstrKind::IncompleteContinue.at(expr.source))
        }
        ast::ExprKind::IntegerPromote(value) => {
            let value = flatten_expr(ctx, builder, cursor, value, IsValue::RequireValue);
            builder.push(cursor, InstrKind::IntegerPromote(value).at(expr.source))
        }
        ast::ExprKind::StaticAssert(..) => todo!(),
        ast::ExprKind::Is(value, variant) => {
            let value = flatten_expr(ctx, builder, cursor, &value, IsValue::RequireValue);
            builder.push(cursor, InstrKind::Is(value, &variant).at(expr.source))
        }
        ast::ExprKind::LabelLiteral(label_name) => {
            builder.push(cursor, InstrKind::LabelLiteral(label_name).at(expr.source))
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
) -> (Joined, Joined) {
    let source = expr.source;
    let void = builder.push(cursor, InstrKind::VoidLiteral.at(source));

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
        void,
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

struct Joined {
    cursor: Cursor,
    instr: Option<InstrRef>,
}

impl Joined {
    pub fn new(cursor: Cursor, instr: Option<InstrRef>) -> Self {
        Self { cursor, instr }
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
    mut incoming: impl ExactSizeIterator<Item = Joined>,
    conform_behavior: Option<ConformBehavior>,
    source: Source,
) -> Joined {
    if incoming.len() == 1 {
        return incoming.next().unwrap();
    }

    let mut joined_bb = builder.new_block();

    Joined {
        instr: Some(existing_basicblock_joining(
            ctx,
            builder,
            &mut joined_bb,
            incoming,
            conform_behavior,
            source,
        )),
        cursor: joined_bb,
    }
}

fn existing_basicblock_joining<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    joined_bb: &mut Cursor,
    incoming: impl IntoIterator<Item = Joined>,
    conform_behavior: Option<ConformBehavior>,
    source: Source,
) -> InstrRef {
    let mut values = Vec::new();

    for mut joined in incoming {
        if builder.has_end(&joined.cursor) {
            continue;
        }

        let value = joined.instr;

        values.push((joined.cursor().basicblock().into_raw(), value));
        builder.push_end(
            joined.cursor(),
            EndInstrKind::Jump(joined_bb.basicblock().into_raw(), value, None).at(source),
        );
    }

    let joined_value = builder.push(
        joined_bb,
        InstrKind::Phi(
            ctx.alloc_slice_fill_iter(values.into_iter()),
            conform_behavior,
        )
        .at(source),
    );

    joined_value
}

fn flatten_condition_inner<'env>(
    ctx: &mut ExecutionCtx<'env>,
    builder: &mut CfgBuilder<'env>,
    cursor: &mut Cursor,
    expr: &'env ast::Expr,
    language: Language,
    join_when_true: &mut Vec<Joined>,
    join_when_false: &mut Vec<Joined>,
    void: InstrRef,
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
                    void,
                ),
                ast::ShortCircuitingBinaryOperator::Or => flatten_condition_inner(
                    ctx,
                    builder,
                    cursor,
                    &bin_op.left,
                    bin_op.conform_behavior.language(),
                    join_when_true,
                    &mut more_to_compute,
                    void,
                ),
            }
        }
        _ => {
            let value = flatten_expr(ctx, builder, cursor, expr, IsValue::RequireValue);
            let condition = builder.push(
                cursor,
                InstrKind::ConformToBool(value, language, None).at(source),
            );
            let (when_true, when_false) = builder.push_branch(condition, cursor, None, source);
            join_when_true.push(Joined::new(when_true, Some(void.clone())));
            join_when_false.push(Joined::new(when_false, Some(void)));
        }
    }
}
