use crate::{
    BasicBinaryOperator, Call, Expr, ExprKind, FillBehavior, Integer, Language,
    ShortCircuitingBinaryOperator, StaticMemberValue, Stmt, StmtKind, Type, TypeArg, UnaryOperator,
    Using,
};
use arena::{Arena, Idx, new_id_with_niche};
use attributes::Privacy;
use source_files::Source;
use std::{collections::HashMap, ffi::CString, fmt::Debug};
use token::Name;

#[derive(Clone, Debug)]
pub struct Cursor {
    position: Option<CursorPosition>,
}

impl Cursor {
    pub fn terminated() -> Self {
        Self { position: None }
    }

    pub fn value(&self) -> Option<NodeRef> {
        if let Some(position) = &self.position {
            Some(position.from)
        } else {
            None
        }
    }

    pub fn is_terminated(&self) -> bool {
        self.position.is_none()
    }

    pub fn is_live(&self) -> bool {
        self.position.is_none()
    }
}

impl From<CursorPosition> for Cursor {
    fn from(value: CursorPosition) -> Self {
        Self {
            position: Some(value),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CursorPosition {
    from: NodeRef,
    edge_index: usize,
}

impl CursorPosition {
    pub fn new(from: NodeRef, edge_index: usize) -> Self {
        Self { from, edge_index }
    }
}

#[derive(Debug)]
pub struct Builder<'const_evals> {
    ordered_nodes: Arena<NodeId, Node>,
    const_evals: &'const_evals mut Arena<ConstEvalId, ConstEval>,
}

impl<'const_evals> Builder<'const_evals> {
    #[must_use]
    pub fn new(
        const_evals: &'const_evals mut Arena<ConstEvalId, ConstEval>,
        source: Source,
    ) -> (Self, Cursor) {
        let mut ordered_nodes = Arena::new();

        let start_ref = ordered_nodes.alloc(Node {
            kind: NodeKind::Start(None),
            source,
        });

        (
            Self {
                ordered_nodes,
                const_evals,
            },
            CursorPosition {
                from: start_ref,
                edge_index: 0,
            }
            .into(),
        )
    }

    #[must_use]
    pub fn get(&self, idx: NodeRef) -> &Node {
        &self.ordered_nodes[idx]
    }

    #[must_use]
    pub fn get_mut(&mut self, idx: NodeRef) -> &mut Node {
        &mut self.ordered_nodes[idx]
    }

    #[must_use]
    pub fn push_sequential(
        &mut self,
        cursor: Cursor,
        node: SequentialNodeKind,
        source: Source,
    ) -> Cursor {
        let Some(position) = cursor.position else {
            return cursor;
        };

        let new_node_ref = self.ordered_nodes.alloc(Node {
            kind: NodeKind::Sequential(SequentialNode {
                kind: node,
                next: None,
            }),
            source,
        });

        connect(&mut self.ordered_nodes, position, new_node_ref);
        CursorPosition::new(new_node_ref, 0).into()
    }

    #[must_use]
    pub fn open_scope(&mut self, cursor: Cursor, source: Source) -> Cursor {
        self.push_sequential(cursor, SequentialNodeKind::OpenScope, source)
    }

    #[must_use]
    pub fn close_scope(&mut self, cursor: Cursor, source: Source) -> Cursor {
        self.push_sequential(cursor, SequentialNodeKind::OpenScope, source)
    }

    #[must_use]
    pub fn push_branch(
        &mut self,
        cursor: Cursor,
        condition: NodeRef,
        source: Source,
    ) -> (Cursor, Cursor) {
        let Some(position) = cursor.position else {
            return (cursor.clone(), cursor);
        };

        let new_node_ref = self.ordered_nodes.alloc(Node {
            kind: NodeKind::Branching(BranchNode {
                condition,
                when_true: None,
                when_false: None,
            }),
            source,
        });

        connect(&mut self.ordered_nodes, position, new_node_ref);

        (
            CursorPosition::new(new_node_ref, 0).into(),
            CursorPosition::new(new_node_ref, 1).into(),
        )
    }

    #[must_use]
    pub fn push_terminating(
        &mut self,
        cursor: Cursor,
        node: TerminatingNode,
        source: Source,
    ) -> Cursor {
        let Some(position) = cursor.position else {
            return cursor;
        };

        let new_node_ref = self.ordered_nodes.alloc(Node {
            kind: NodeKind::Terminating(node),
            source,
        });

        connect(&mut self.ordered_nodes, position, new_node_ref);
        Cursor::terminated()
    }

    #[must_use]
    pub fn push_join(
        &mut self,
        cursor_a: Cursor,
        a_gives: Option<NodeRef>,
        cursor_b: Cursor,
        b_gives: Option<NodeRef>,
        source: Source,
    ) -> Cursor {
        if (cursor_a.is_terminated() && cursor_b.is_terminated())
            || (a_gives.is_none() && b_gives.is_none())
        {
            return Cursor::terminated();
        }

        if let (Some(a_position), Some(b_position), Some(a_gives), Some(b_gives)) =
            (&cursor_a.position, &cursor_b.position, a_gives, b_gives)
        {
            let new_node_ref = self.ordered_nodes.alloc(Node {
                kind: NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::Join2(
                        a_position.clone(),
                        a_gives,
                        b_position.clone(),
                        b_gives,
                    ),
                    next: None,
                }),
                source,
            });

            connect(&mut self.ordered_nodes, a_position.clone(), new_node_ref);
            connect(&mut self.ordered_nodes, b_position.clone(), new_node_ref);
            return CursorPosition::new(new_node_ref, 0).into();
        }

        if cursor_a.is_live() {
            if let Some(a_gives) = a_gives {
                return self.push_sequential(cursor_a, SequentialNodeKind::Join1(a_gives), source);
            }
        } else if cursor_b.is_live() {
            if let Some(b_gives) = b_gives {
                return self.push_sequential(cursor_b, SequentialNodeKind::Join1(b_gives), source);
            }
        }

        Cursor::terminated()
    }

    #[must_use]
    pub fn push_join_n(
        &mut self,
        incoming: Vec<(Cursor, Option<NodeRef>)>,
        source: Source,
    ) -> Cursor {
        let incoming: Vec<_> = incoming
            .into_iter()
            .filter_map(|(cursor, value)| cursor.position.zip(value))
            .collect();

        match incoming.len() {
            0 => Cursor::terminated(),
            1 => {
                let mut incoming = incoming.into_iter();
                let (position, gives_value) = incoming.next().unwrap();

                return self.push_sequential(
                    position.into(),
                    SequentialNodeKind::Join1(gives_value),
                    source,
                );
            }
            2 => {
                let mut incoming = incoming.into_iter();
                let (a_position, a_gives) = incoming.next().unwrap();
                let (b_position, b_gives) = incoming.next().unwrap();

                let new_node_ref = self.ordered_nodes.alloc(Node {
                    kind: NodeKind::Sequential(SequentialNode {
                        kind: SequentialNodeKind::Join2(
                            a_position.clone(),
                            a_gives,
                            b_position.clone(),
                            b_gives,
                        ),
                        next: None,
                    }),
                    source,
                });

                connect(&mut self.ordered_nodes, a_position, new_node_ref);
                connect(&mut self.ordered_nodes, b_position, new_node_ref);
                return CursorPosition::new(new_node_ref, 0).into();
            }
            _ => {
                let edges: Vec<_> = incoming
                    .iter()
                    .map(|(position, _)| position)
                    .cloned()
                    .collect();

                let new_node_ref = self.ordered_nodes.alloc(Node {
                    kind: NodeKind::Sequential(SequentialNode {
                        kind: SequentialNodeKind::JoinN(incoming),
                        next: None,
                    }),
                    source,
                });

                for position in edges {
                    connect(&mut self.ordered_nodes, position, new_node_ref);
                }
                return CursorPosition::new(new_node_ref, 0).into();
            }
        }
    }

    #[must_use]
    pub fn const_eval(&mut self, expr: Expr) -> ConstEvalRef {
        let cfg = {
            let source = expr.source;
            let (mut builder, mut cursor) = Builder::new(self.const_evals, source);
            cursor = flatten_expr(&mut builder, cursor, expr, IsValue::RequireValue);
            let value = cursor.value();
            let _ = builder.push_terminating(cursor, TerminatingNode::Computed(value), source);
            UntypedCfg {
                ordered_nodes: builder.ordered_nodes,
            }
        };

        self.const_evals.alloc(ConstEval {
            context: HashMap::default(),
            cfg,
        })
    }
}

new_id_with_niche!(ConstEvalId, u64);

pub type ConstEvalRef = Idx<ConstEvalId, ConstEval>;

#[derive(Clone, Debug)]
pub struct ConstEval {
    context: HashMap<String, Vec<SymbolBinding>>,
    cfg: UntypedCfg,
}

#[derive(Clone, Debug)]
pub struct SymbolBinding {
    pub symbol: SymbolRef,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum SymbolRef {
    ConstEval(ConstEvalRef),
    /*
    Global(GlobalRef),
    Func(FuncRef),
    ExprAlias(ExprAliasRef),
    TypeAlias(TypeAliasRef),
    Trait(TraitRef),
    Impl(ImplRef),
    */
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IsValue {
    RequireValue,
    NeglectValue,
}

pub fn flatten_func_ignore_const_evals(stmts: &Vec<Stmt>, source: Source) -> UntypedCfg {
    let mut const_evals = Arena::new();
    flatten_func(stmts, &mut const_evals, source)
}

pub fn flatten_func(
    stmts: &Vec<Stmt>,
    const_evals: &mut Arena<ConstEvalId, ConstEval>,
    source: Source,
) -> UntypedCfg {
    // NOTE: Eventually we won't need this cloning
    let stmts = stmts.clone();

    let (mut builder, cursor) = Builder::new(const_evals, source);
    let _ = flatten_stmts(&mut builder, cursor, stmts, IsValue::NeglectValue);

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
            let left_source = bin_op.left.source;
            let right_source = bin_op.right.source;

            cursor = flatten_expr(builder, cursor, bin_op.left, IsValue::RequireValue);
            cursor = builder.push_sequential(
                cursor,
                SequentialNodeKind::ConformToBool(bin_op.language),
                left_source,
            );

            let Some(left) = cursor.value() else {
                return cursor;
            };

            let (mut when_true, mut when_false) = builder.push_branch(cursor, left, expr.source);

            // NOTE: For C, the pre-conforming value should be the result, but we don't do that yet
            let (true_gives, false_gives) = match bin_op.operator {
                ShortCircuitingBinaryOperator::And => {
                    // When true, result should be right hand side
                    when_true = builder.open_scope(when_true, expr.source);
                    when_true =
                        flatten_expr(builder, when_true, bin_op.right, IsValue::RequireValue);
                    when_true = builder.push_sequential(
                        when_true,
                        SequentialNodeKind::ConformToBool(bin_op.language),
                        right_source,
                    );
                    let right = when_true.value();
                    when_true = builder.close_scope(when_true, expr.source);

                    // When false, result should be left hand side
                    (right, Some(left))
                }
                ShortCircuitingBinaryOperator::Or => {
                    // When true, result should be left hand side

                    // When false, result should be right hand side
                    when_false = builder.open_scope(when_false, expr.source);
                    when_false =
                        flatten_expr(builder, when_false, bin_op.right, IsValue::RequireValue);
                    when_false = builder.push_sequential(
                        when_false,
                        SequentialNodeKind::ConformToBool(bin_op.language),
                        right_source,
                    );
                    let right = when_false.value();
                    when_false = builder.close_scope(when_false, expr.source);
                    (Some(left), right)
                }
            };

            builder.push_join(when_true, true_gives, when_false, false_gives, expr.source)
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
                cursor = flatten_expr(builder, cursor, condition, IsValue::RequireValue);

                let Some(condition) = cursor.value() else {
                    return cursor;
                };

                let (mut when_true, mut when_false) =
                    builder.push_branch(cursor, condition, expr.source);

                when_true = flatten_stmts(builder, when_true, block.stmts, is_value);
                let value = no_result.or_else(|| when_true.value());

                // Close scope after inner block or if not selected
                when_true = builder.close_scope(when_true, expr.source);
                when_false = builder.close_scope(when_false, expr.source);

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

            cursor = builder.push_join_n(incoming, expr.source);
            builder.push_sequential(cursor, SequentialNodeKind::CloseScope, expr.source)
        }
        ExprKind::While(while_loop) => {
            cursor = builder.push_sequential(cursor, SequentialNodeKind::Void, expr.source);

            let Some(void_result) = cursor.value() else {
                return cursor;
            };

            cursor = builder.open_scope(cursor, expr.source);

            cursor = flatten_expr(builder, cursor, while_loop.condition, IsValue::RequireValue);
            let Some(condition) = cursor.value() else {
                return cursor;
            };

            let (mut when_true, mut when_false) =
                builder.push_branch(cursor, condition, expr.source);

            when_true = flatten_stmts(
                builder,
                when_true,
                while_loop.block.stmts,
                IsValue::NeglectValue,
            );

            when_true = builder.close_scope(when_true, expr.source);
            when_false = builder.close_scope(when_false, expr.source);

            builder.push_join(
                when_true,
                Some(void_result),
                when_false,
                Some(void_result),
                expr.source,
            )
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

new_id_with_niche!(NodeId, u64);
pub type NodeRef = Idx<NodeId, Node>;

#[derive(Clone, Debug)]
pub struct UntypedCfg {
    pub ordered_nodes: Arena<NodeId, Node>,
}

#[derive(Clone)]
pub struct Node {
    kind: NodeKind,
    source: Source,
}

impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

pub fn connect(nodes: &mut Arena<NodeId, Node>, from: CursorPosition, to: NodeRef) {
    let node = &mut nodes[from.from];

    match &mut node.kind {
        NodeKind::Start(next) => {
            assert_eq!(from.edge_index, 0);
            *next = Some(to);
        }
        NodeKind::Sequential(sequential_node) => {
            assert_eq!(from.edge_index, 0);
            sequential_node.next = Some(to);
        }
        NodeKind::Branching(branch) => match from.edge_index {
            0 => branch.when_true = Some(to),
            1 => branch.when_false = Some(to),
            _ => panic!("invalid from edge index for branching node"),
        },
        NodeKind::Terminating(_) => panic!("cannot connect terminationg node"),
    }
}

#[derive(Clone, Debug)]
pub struct SequentialNode {
    kind: SequentialNodeKind,
    next: Option<NodeRef>,
}

#[derive(Clone, Debug)]
pub struct Join {
    parent_a: NodeRef,
    gives_a: NodeRef,
    parent_b: NodeRef,
    gives_b: NodeRef,
}

#[derive(Clone, Debug)]
pub enum SequentialNodeKind {
    Join1(NodeRef),
    Join2(CursorPosition, NodeRef, CursorPosition, NodeRef),
    JoinN(Vec<(CursorPosition, NodeRef)>),
    Const(UntypedCfg),
    Name(Name),
    OpenScope,
    CloseScope,
    NewVariable(String, Type),
    Declare(String, Type, Option<NodeRef>),
    Assign(NodeRef, NodeRef),
    BinOp(NodeRef, BasicBinaryOperator, NodeRef),
    Boolean(bool),
    Integer(Integer),
    Float(f64),
    AsciiChar(u8),
    Utf8Char(String),
    String(String),
    NullTerminatedString(CString),
    Null,
    Void,
    Call(Box<NodeCall>),
    DeclareAssign(String, NodeRef),
    Member(NodeRef, String, Privacy),
    ArrayAccess(NodeRef, NodeRef),
    StructLiteral(Box<NodeStructLiteral>),
    UnaryOperation(UnaryOperator, NodeRef),
    StaticMemberValue(Box<StaticMemberValue>),
    StaticMemberCall(Box<NodeStaticMemberCall>),
    SizeOf(Type),
    SizeOfValue(NodeRef),
    InterpreterSyscall(NodeInterpreterSyscall),
    IntegerPromote(NodeRef),
    StaticAssert(ConstEvalRef, Option<String>),
    ConformToBool(Language),
}

#[derive(Clone, Debug)]
pub struct NodeCall {
    name: Name,
    args: Vec<NodeRef>,
    expected_to_return: Option<Type>,
    generics: Vec<NodeTypeArg>,
    using: Vec<Using>,
}

#[derive(Clone, Debug)]
pub enum NodeTypeArg {
    Type(Type),
    Expr(ConstEvalRef),
}

#[derive(Clone, Debug)]
pub struct NodeStaticMemberCall {
    pub subject: Type,
    pub call: NodeCall,
    pub call_source: Source,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct NodeStructLiteral {
    pub ast_type: Type,
    pub fields: Vec<NodeFieldInitializer>,
    pub fill_behavior: FillBehavior,
    pub language: Language,
}

#[derive(Clone, Debug)]
pub struct NodeFieldInitializer {
    pub name: Option<String>,
    pub value: NodeRef,
}

#[derive(Clone, Debug)]
pub struct NodeDeclareAssign {
    pub name: String,
    pub value: NodeRef,
}

#[derive(Clone, Debug)]
pub struct NodeInterpreterSyscall {
    pub kind: interpreter_api::Syscall,
    pub args: Vec<(Type, NodeRef)>,
    pub result_type: Type,
}

#[derive(Clone, Debug)]
pub struct BranchNode {
    condition: NodeRef,
    when_true: Option<NodeRef>,
    when_false: Option<NodeRef>,
}

#[derive(Clone, Debug)]
pub enum TerminatingNode {
    Return(Option<NodeRef>),
    Computed(Option<NodeRef>),
    Break,
    Continue,
}

#[derive(Clone, Debug)]
pub enum NodeKind {
    Start(Option<NodeRef>),
    Sequential(SequentialNode),
    Branching(BranchNode),
    Terminating(TerminatingNode),
}
