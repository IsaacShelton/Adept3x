use crate::resolved::{self, VariableStorage, VariableStorageKey};
use bit_vec::BitVec;

struct VariableUsageSet {
    declared: BitVec,
    used: BitVec,
}

impl VariableUsageSet {
    pub fn new(count: usize) -> Self {
        Self {
            declared: BitVec::from_elem(count, false),
            used: BitVec::from_elem(count, false),
        }
    }

    pub fn declare(&mut self, storage: VariableStorageKey) {
        self.declared.set(storage.index, true);
    }

    pub fn mark_used(&mut self, storage: VariableStorageKey) {
        self.used.set(storage.index, true);
    }

    pub fn union_with(&mut self, other: &Self) {
        self.used.or(&other.used);
        self.declared.or(&other.declared);
    }

    pub fn iter_used(&self) -> impl Iterator<Item = bool> + '_ {
        self.used.iter()
    }

    pub fn iter_declared(&self) -> impl Iterator<Item = bool> + '_ {
        self.declared.iter()
    }
}

pub fn insert_drops(function: &mut resolved::Function) {
    // Search through statements top to bottom, and record which statement each variable storage
    // key is last used by

    insert_drops_for_stmts(&mut function.stmts, &function.variables);
}

fn insert_drops_for_stmts(
    stmts: &mut Vec<resolved::Stmt>,
    variables: &VariableStorage,
) -> VariableUsageSet {
    let count = variables.count();
    let mut last_use_in_this_scope = vec![0 as usize; count];
    let mut scope = VariableUsageSet::new(count.try_into().unwrap());

    for (i, stmt) in stmts.iter().enumerate() {
        let mentioned = match &stmt.kind {
            resolved::StmtKind::Return(expr, _) => {
                if let Some(expr) = expr {
                    insert_drops_for_expr(count, expr)
                } else {
                    VariableUsageSet::new(count)
                }
            }
            resolved::StmtKind::Expr(expr) => insert_drops_for_expr(count, &expr.expr),
            resolved::StmtKind::Declaration(declaration) => {
                scope.declare(declaration.key);
                last_use_in_this_scope[declaration.key.index] = i;

                if let Some(expr) = &declaration.value {
                    insert_drops_for_expr(count, expr)
                } else {
                    VariableUsageSet::new(count)
                }
            }
            resolved::StmtKind::Assignment(assignment) => {
                let mut mentioned = insert_drops_for_expr(count, &assignment.value);
                mentioned.union_with(&insert_drops_for_destination(count));
                mentioned
            }
        };

        scope.union_with(&mentioned);

        for (mention_index, did_declare) in mentioned.iter_declared().enumerate() {
            if did_declare {
                last_use_in_this_scope[mention_index] = i;
            }
        }

        for (mention_index, did_mention) in mentioned.iter_used().enumerate() {
            if did_mention {
                last_use_in_this_scope[mention_index] = i;
            }
        }
    }

    let mut drops = Vec::new();

    for (variable_index, did_declare) in scope.iter_declared().enumerate() {
        if !did_declare {
            continue;
        }

        let should_drop_after = last_use_in_this_scope[variable_index];
        drops.push((should_drop_after, variable_index))
    }

    drops.sort_by(
        |(a_after_stmt, a_variable_index), (b_after_stmt, b_variable_index)| {
            a_after_stmt
                .cmp(b_after_stmt)
                .then(a_variable_index.cmp(b_variable_index))
        },
    );

    scope
}

fn insert_drops_for_expr(variable_count: usize, expr: &resolved::Expr) -> VariableUsageSet {
    let mut mini_scope = VariableUsageSet::new(variable_count);

    match &expr.kind {
        resolved::ExprKind::Variable(_) => (),
        resolved::ExprKind::GlobalVariable(_) => (),
        resolved::ExprKind::BooleanLiteral(_)
        | resolved::ExprKind::IntegerLiteral(_)
        | resolved::ExprKind::Integer { .. }
        | resolved::ExprKind::Float(..)
        | resolved::ExprKind::String(..)
        | resolved::ExprKind::NullTerminatedString(..) => (),
        resolved::ExprKind::Call(_) => (),
        resolved::ExprKind::DeclareAssign(declare_assign) => {
            mini_scope.declare(declare_assign.key);
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &declare_assign.value,
            ));
        }
        resolved::ExprKind::BasicBinaryOperation(_) => (),
        resolved::ExprKind::ShortCircuitingBinaryOperation(_) => (),
        resolved::ExprKind::IntegerExtend(_, _) => (),
        resolved::ExprKind::FloatExtend(_, _) => (),
        resolved::ExprKind::Member { .. } => (),
        resolved::ExprKind::StructureLiteral { .. } => (),
        resolved::ExprKind::UnaryOperation(_) => (),
        resolved::ExprKind::Conditional(_) => (),
        resolved::ExprKind::While(_) => (),
        resolved::ExprKind::ArrayAccess(_) => (),
    }

    mini_scope
}

fn insert_drops_for_destination(variable_count: usize) -> VariableUsageSet {
    let used = VariableUsageSet::new(variable_count);

    used
}
