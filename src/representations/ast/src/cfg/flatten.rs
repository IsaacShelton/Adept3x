use super::{
    ConstEval, ConstEvalId, IsValue, Node, NodeCall, NodeFieldInitializer, NodeInterpreterSyscall,
    NodeKind, NodeRef, NodeStaticMemberCall, NodeStructLiteral, NodeTypeArg, SequentialNode,
    SequentialNodeKind, TerminatingNode, UntypedCfg,
    builder::Builder,
    connect,
    cursor::{Cursor, CursorPosition},
};
use crate::{
    Call, ConformBehavior, Expr, ExprKind, Language, ShortCircuitingBinaryOperator, Stmt, StmtKind,
    TypeArg,
};
use arena::Arena;
use smallvec::smallvec;
use source_files::Source;

pub fn flatten_func_ignore_const_evals(stmts: Vec<Stmt>, source: Source) -> UntypedCfg {
    let mut const_evals = Arena::new();
    flatten_func(stmts, &mut const_evals, source)
}

pub fn flatten_func(
    stmts: Vec<Stmt>,
    const_evals: &mut Arena<ConstEvalId, ConstEval>,
    source: Source,
) -> UntypedCfg {
    let stmts = stmts.clone();

    let (mut builder, cursor) = Builder::new(const_evals, source);
    let cursor = flatten_stmts(&mut builder, cursor, stmts, IsValue::NeglectValue);
    let _ = builder.push_terminating(cursor, TerminatingNode::Return(None), source);

    UntypedCfg {
        ordered_nodes: builder.ordered_nodes,
    }
}

#[must_use]
pub fn flatten_stmts(
    builder: &mut Builder,
    mut cursor: Cursor,
    stmts: Vec<Stmt>,
    is_value: IsValue,
) -> Cursor {
    let length = stmts.len();
    for (i, stmt) in stmts.into_iter().enumerate() {
        if i + 1 == length {
            cursor = flatten_stmt(builder, cursor, stmt, is_value);
        } else {
            cursor = flatten_stmt(builder, cursor, stmt, IsValue::NeglectValue);
        }
    }
    cursor
}

#[must_use]
pub fn flatten_stmt(
    builder: &mut Builder,
    cursor: Cursor,
    stmt: Stmt,
    is_value: IsValue,
) -> Cursor {
    match stmt.kind {
        StmtKind::Return(expr) => {
            let cursor = if let Some(expr) = expr {
                flatten_expr(builder, cursor, expr, IsValue::RequireValue)
            } else {
                cursor
            };

            let value = cursor.value();
            builder.push_terminating(cursor, TerminatingNode::Return(value), stmt.source)
        }
        StmtKind::Expr(expr) => flatten_expr(builder, cursor, expr, is_value),
        StmtKind::Declaration(declaration) => {
            let cursor = if let Some(value) = declaration.initial_value {
                flatten_expr(builder, cursor, value, IsValue::RequireValue)
            } else {
                cursor
            };

            let value = cursor.value();
            builder.push_sequential(
                cursor,
                SequentialNodeKind::Declare(declaration.name, declaration.ast_type, value),
                stmt.source,
            )
        }
        StmtKind::Assignment(assignment) => {
            let cursor = flatten_expr(builder, cursor, assignment.value, IsValue::RequireValue);
            let left = cursor.value();

            let cursor = flatten_expr(
                builder,
                cursor,
                assignment.destination,
                IsValue::RequireValue,
            );
            let right = cursor.value();

            if let Some((left, right)) = left.zip(right) {
                builder.push_sequential(
                    cursor,
                    SequentialNodeKind::Assign(left, right),
                    stmt.source,
                )
            } else {
                cursor
            }
        }
    }
}

#[must_use]
pub fn flatten_condition(
    builder: &mut Builder,
    mut cursor: Cursor,
    expr: Expr,
    language: Language,
    conform_behavior: Option<ConformBehavior>,
) -> (Cursor, Cursor) {
    let source = expr.source;
    let mut when_true = vec![];
    let mut when_false = vec![];

    cursor = builder.push_sequential(cursor, SequentialNodeKind::Void, source);
    let Some(void) = cursor.value() else {
        return (Cursor::terminated(), Cursor::terminated());
    };

    flatten_condition_inner(
        builder,
        cursor,
        expr,
        language,
        &mut when_true,
        &mut when_false,
        void,
    );

    let when_true = builder.push_join_n(when_true, conform_behavior, source);
    let when_false = builder.push_join_n(when_false, conform_behavior, source);
    (when_true, when_false)
}

pub fn flatten_condition_inner(
    builder: &mut Builder,
    mut cursor: Cursor,
    expr: Expr,
    language: Language,
    join_when_true: &mut Vec<(Cursor, Option<NodeRef>)>,
    join_when_false: &mut Vec<(Cursor, Option<NodeRef>)>,
    void: NodeRef,
) {
    let source = expr.source;

    match expr.kind {
        ExprKind::ShortCircuitingBinaryOperation(bin_op) => {
            let mut more_to_compute = Vec::new();

            match bin_op.operator {
                ShortCircuitingBinaryOperator::And => flatten_condition_inner(
                    builder,
                    cursor,
                    bin_op.left,
                    bin_op.conform_behavior.language(),
                    &mut more_to_compute,
                    join_when_false,
                    void,
                ),
                ShortCircuitingBinaryOperator::Or => flatten_condition_inner(
                    builder,
                    cursor,
                    bin_op.left,
                    bin_op.conform_behavior.language(),
                    join_when_true,
                    &mut more_to_compute,
                    void,
                ),
            };

            cursor = builder.push_join_n(more_to_compute, Some(bin_op.conform_behavior), source);

            flatten_condition_inner(
                builder,
                cursor,
                bin_op.right,
                bin_op.conform_behavior.language(),
                join_when_true,
                join_when_false,
                void,
            );
        }
        _ => {
            cursor = flatten_expr(builder, cursor, expr, IsValue::RequireValue);

            let Some(value) = cursor.value() else {
                return;
            };

            cursor = builder.push_sequential(
                cursor,
                SequentialNodeKind::ConformToBool(value, language),
                source,
            );

            let Some(value) = cursor.value() else {
                return;
            };

            let (when_true, when_false) = builder.push_branch(cursor, value, source);
            join_when_true.push((when_true, Some(void)));
            join_when_false.push((when_false, Some(void)));
        }
    }
}

#[must_use]
pub fn flatten_expr(
    builder: &mut Builder,
    mut cursor: Cursor,
    expr: Expr,
    is_value: IsValue,
) -> Cursor {
    match expr.kind {
        ExprKind::Variable(name) => {
            builder.push_sequential(cursor, SequentialNodeKind::Name(name), expr.source)
        }
        ExprKind::Boolean(value) => {
            builder.push_sequential(cursor, SequentialNodeKind::Boolean(value), expr.source)
        }
        ExprKind::Integer(integer) => {
            builder.push_sequential(cursor, SequentialNodeKind::Integer(integer), expr.source)
        }
        ExprKind::Float(float) => {
            builder.push_sequential(cursor, SequentialNodeKind::Float(float), expr.source)
        }
        ExprKind::Char(char) => {
            builder.push_sequential(cursor, SequentialNodeKind::Utf8Char(char), expr.source)
        }
        ExprKind::String(string) => {
            builder.push_sequential(cursor, SequentialNodeKind::String(string), expr.source)
        }
        ExprKind::NullTerminatedString(cstring) => builder.push_sequential(
            cursor,
            SequentialNodeKind::NullTerminatedString(cstring),
            expr.source,
        ),
        ExprKind::CharLiteral(ascii_char) => builder.push_sequential(
            cursor,
            SequentialNodeKind::AsciiChar(ascii_char),
            expr.source,
        ),
        ExprKind::Null => builder.push_sequential(cursor, SequentialNodeKind::Null, expr.source),
        ExprKind::Call(call) => {
            let (cursor, call) = match flatten_call(builder, cursor, *call) {
                Ok(values) => values,
                Err(cursor) => return cursor,
            };

            builder.push_sequential(
                cursor,
                SequentialNodeKind::Call(Box::new(call)),
                expr.source,
            )
        }
        ExprKind::DeclareAssign(declare_assign) => {
            let cursor = flatten_expr(builder, cursor, declare_assign.value, IsValue::RequireValue);
            if let Some(value) = cursor.value() {
                builder.push_sequential(
                    cursor,
                    SequentialNodeKind::DeclareAssign(declare_assign.name, value),
                    expr.source,
                )
            } else {
                cursor
            }
        }
        ExprKind::BasicBinaryOperation(bin_op) => {
            cursor = flatten_expr(builder, cursor, bin_op.left, IsValue::RequireValue);
            let left = cursor.value();
            cursor = flatten_expr(builder, cursor, bin_op.right, IsValue::RequireValue);
            let right = cursor.value();

            if let Some((left, right)) = left.zip(right) {
                builder.push_sequential(
                    cursor,
                    SequentialNodeKind::BinOp(left, bin_op.operator, right),
                    expr.source,
                )
            } else {
                cursor
            }
        }
        ExprKind::ShortCircuitingBinaryOperation(bin_op) => {
            let right_source = bin_op.right.source;

            let (mut when_true, mut when_false) = flatten_condition(
                builder,
                cursor,
                bin_op.left,
                bin_op.conform_behavior.language(),
                Some(bin_op.conform_behavior),
            );

            let when_more_calc = match bin_op.operator {
                ShortCircuitingBinaryOperator::And => &mut when_true,
                ShortCircuitingBinaryOperator::Or => &mut when_false,
            };

            // NOTE: For C, the pre-conforming value should be the result, but we don't do that yet
            *when_more_calc = builder.open_scope(*when_more_calc, expr.source);
            *when_more_calc = flatten_expr(
                builder,
                *when_more_calc,
                bin_op.right,
                IsValue::RequireValue,
            );

            if let Some(inner) = when_more_calc.value() {
                *when_more_calc = builder.push_sequential(
                    *when_more_calc,
                    SequentialNodeKind::ConformToBool(inner, bin_op.conform_behavior.language()),
                    right_source,
                );
            }

            *when_more_calc = builder.close_scope(*when_more_calc, expr.source);

            builder.push_join(
                when_true,
                when_true.value(),
                when_false,
                when_false.value(),
                Some(bin_op.conform_behavior),
                expr.source,
            )
        }
        ExprKind::Member(subject, member, privacy) => {
            cursor = flatten_expr(builder, cursor, *subject, IsValue::RequireValue);
            let subject = cursor.value();

            if let Some(subject) = subject {
                builder.push_sequential(
                    cursor,
                    SequentialNodeKind::Member(subject, member, privacy),
                    expr.source,
                )
            } else {
                cursor
            }
        }
        ExprKind::ArrayAccess(array_access) => {
            cursor = flatten_expr(builder, cursor, array_access.subject, IsValue::RequireValue);
            let subject = cursor.value();

            cursor = flatten_expr(builder, cursor, array_access.index, IsValue::RequireValue);
            let index = cursor.value();

            if let Some((subject, index)) = subject.zip(index) {
                builder.push_sequential(
                    cursor,
                    SequentialNodeKind::ArrayAccess(subject, index),
                    expr.source,
                )
            } else {
                cursor
            }
        }
        ExprKind::StructLiteral(struct_literal) => {
            let mut fields = Vec::with_capacity(struct_literal.fields.len());

            for field in struct_literal.fields {
                cursor = flatten_expr(builder, cursor, field.value, IsValue::RequireValue);

                let Some(value) = cursor.value() else {
                    return Cursor::terminated();
                };

                fields.push(NodeFieldInitializer {
                    name: field.name,
                    value,
                });
            }

            builder.push_sequential(
                cursor,
                SequentialNodeKind::StructLiteral(Box::new(NodeStructLiteral {
                    ast_type: struct_literal.ast_type,
                    fields,
                    fill_behavior: struct_literal.fill_behavior,
                    language: struct_literal.language,
                })),
                expr.source,
            )
        }
        ExprKind::UnaryOperation(unary_operation) => {
            cursor = flatten_expr(
                builder,
                cursor,
                unary_operation.inner,
                IsValue::RequireValue,
            );
            let inner = cursor.value();

            if let Some(inner) = inner {
                builder.push_sequential(
                    cursor,
                    SequentialNodeKind::UnaryOperation(unary_operation.operator, inner),
                    expr.source,
                )
            } else {
                cursor
            }
        }
        ExprKind::Conditional(conditional) => {
            let mut incoming = vec![];

            let no_result = match is_value {
                IsValue::RequireValue => None,
                IsValue::NeglectValue => {
                    cursor = builder.push_sequential(cursor, SequentialNodeKind::Void, expr.source);
                    let Some(value) = cursor.value() else {
                        return cursor;
                    };
                    Some(value)
                }
            };

            for (condition, block) in conditional.conditions {
                // Open scope before evaluating condition
                cursor = builder.open_scope(cursor, expr.source);

                let (mut when_true, mut when_false) = flatten_condition(
                    builder,
                    cursor,
                    condition,
                    conditional.conform_behavior.language(),
                    Some(conditional.conform_behavior),
                );

                when_false = builder.close_scope(when_false, expr.source);

                when_true = flatten_stmts(builder, when_true, block.stmts, is_value);
                when_true = builder.close_scope(when_true, expr.source);
                let value = no_result.or_else(|| when_true.value());

                incoming.push((when_true.clone(), value));
                cursor = when_false;
            }

            if let Some(otherwise) = conditional.otherwise {
                cursor = builder.open_scope(cursor, expr.source);
                cursor = flatten_stmts(builder, cursor, otherwise.stmts, is_value);
                let value = no_result.or_else(|| cursor.value());
                cursor = builder.close_scope(cursor, expr.source);

                incoming.push((cursor, value));
            } else {
                let no_result = if let Some(no_result) = no_result {
                    Some(no_result)
                } else {
                    cursor = builder.push_sequential(cursor, SequentialNodeKind::Void, expr.source);
                    cursor.value()
                };

                incoming.push((cursor.clone(), no_result));
            }

            builder.push_join_n(incoming, Some(conditional.conform_behavior), expr.source)
        }
        ExprKind::While(while_loop) => {
            cursor = builder.push_sequential(cursor, SequentialNodeKind::Void, expr.source);

            let Some(void_result) = cursor.value() else {
                return cursor;
            };

            let Some(start_position) = cursor.position else {
                return cursor;
            };

            let repeat_node_ref = builder.ordered_nodes.alloc(Node {
                kind: NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::JoinN(smallvec![(start_position, void_result)], None),
                    next: None,
                }),
                source: expr.source,
            });

            connect(&mut builder.ordered_nodes, start_position, repeat_node_ref);
            cursor = CursorPosition::new(repeat_node_ref, 0).into();

            cursor = builder.open_scope(cursor, expr.source);

            cursor = flatten_expr(builder, cursor, while_loop.condition, IsValue::RequireValue);
            let Some(condition) = cursor.value() else {
                return cursor;
            };

            let (mut when_true, when_false) = builder.push_branch(cursor, condition, expr.source);

            when_true = flatten_stmts(
                builder,
                when_true,
                while_loop.block.stmts,
                IsValue::NeglectValue,
            );

            when_true = builder.close_scope(when_true, expr.source);

            if let Some(end_position) = when_true.position {
                connect(&mut builder.ordered_nodes, end_position, repeat_node_ref);
                match &mut builder.ordered_nodes[repeat_node_ref].kind {
                    NodeKind::Sequential(sequential_node) => match &mut sequential_node.kind {
                        SequentialNodeKind::JoinN(items, _) => {
                            items.push((end_position, void_result));
                        }
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                }
            }

            builder.close_scope(when_false, expr.source)
        }
        ExprKind::StaticMemberValue(static_member_value) => builder.push_sequential(
            cursor,
            SequentialNodeKind::StaticMemberValue(static_member_value),
            expr.source,
        ),
        ExprKind::StaticMemberCall(static_member_call) => {
            let (cursor, call) = match flatten_call(builder, cursor, static_member_call.call) {
                Ok(values) => values,
                Err(cursor) => return cursor,
            };

            builder.push_sequential(
                cursor,
                SequentialNodeKind::StaticMemberCall(Box::new(NodeStaticMemberCall {
                    subject: static_member_call.subject,
                    call,
                    call_source: static_member_call.call_source,
                    source: static_member_call.source,
                })),
                static_member_call.source,
            )
        }
        ExprKind::SizeOf(ty) => {
            builder.push_sequential(cursor, SequentialNodeKind::SizeOf(*ty), expr.source)
        }
        ExprKind::SizeOfValue(of_value) => {
            cursor = flatten_expr(builder, cursor, *of_value, IsValue::RequireValue);
            let Some(value) = cursor.value() else {
                return cursor;
            };
            builder.push_sequential(cursor, SequentialNodeKind::SizeOfValue(value), expr.source)
        }
        ExprKind::InterpreterSyscall(syscall) => {
            let mut args = Vec::with_capacity(syscall.args.len());
            for (arg_type, arg) in syscall.args {
                cursor = flatten_expr(builder, cursor, arg, IsValue::RequireValue);

                let Some(value) = cursor.value() else {
                    return cursor;
                };

                args.push((arg_type, value));
            }

            builder.push_sequential(
                cursor,
                SequentialNodeKind::InterpreterSyscall(NodeInterpreterSyscall {
                    kind: syscall.kind,
                    args,
                    result_type: syscall.result_type,
                }),
                expr.source,
            )
        }
        ExprKind::Break => builder.push_terminating(cursor, TerminatingNode::Break, expr.source),
        ExprKind::Continue => {
            builder.push_terminating(cursor, TerminatingNode::Continue, expr.source)
        }
        ExprKind::IntegerPromote(value) => {
            cursor = flatten_expr(builder, cursor, *value, IsValue::RequireValue);
            let Some(value) = cursor.value() else {
                return cursor;
            };

            builder.push_sequential(
                cursor,
                SequentialNodeKind::IntegerPromote(value),
                expr.source,
            )
        }
        ExprKind::StaticAssert(value, message) => {
            let condition = builder.const_eval(*value);
            builder.push_sequential(
                cursor,
                SequentialNodeKind::StaticAssert(condition, message),
                expr.source,
            )
        }
        ExprKind::Is(value, variant) => {
            cursor = flatten_expr(builder, cursor, *value, IsValue::RequireValue);
            let Some(value) = cursor.value() else {
                return cursor;
            };

            builder.push_sequential(cursor, SequentialNodeKind::Is(value, variant), expr.source)
        }
    }
}

fn flatten_call(
    builder: &mut Builder,
    mut cursor: Cursor,
    call: Call,
) -> Result<(Cursor, NodeCall), Cursor> {
    let mut args = Vec::with_capacity(call.args.len());
    for arg in call.args {
        cursor = flatten_expr(builder, cursor, arg, IsValue::RequireValue);

        if let Some(value) = cursor.value() {
            args.push(value);
        } else {
            return Err(cursor);
        }
    }

    let mut generics = Vec::with_capacity(call.generics.len());
    for type_arg in call.generics {
        generics.push(match type_arg {
            TypeArg::Type(ty) => NodeTypeArg::Type(ty),
            TypeArg::Expr(expr) => NodeTypeArg::Expr(builder.const_eval(expr)),
        });
    }

    Ok((
        cursor,
        NodeCall {
            name: call.name,
            args,
            expected_to_return: call.expected_to_return,
            generics,
            using: call.using,
        },
    ))
}
