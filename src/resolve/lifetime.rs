use crate::resolved::{
    Destination, DestinationKind, Drops, Expr, ExprKind, Function, Stmt, StmtKind,
    VariableStorageKey,
};
use bit_vec::BitVec;

// Temporary logging function for testing
macro_rules! lifetime_log {
    ($($rest:tt)*) => {
        if false {
            std::println!($($rest)*)
        }
    }
}

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

    pub fn iter_used(
        &self,
    ) -> impl Iterator<Item = bool> + DoubleEndedIterator + ExactSizeIterator + '_ {
        self.used.iter()
    }

    pub fn iter_declared(
        &self,
    ) -> impl Iterator<Item = bool> + DoubleEndedIterator + ExactSizeIterator + '_ {
        self.declared.iter()
    }
}

#[derive(Clone, Debug)]
struct InsertDropsCtx {
    variables_count: usize,
    depth: i32,
    message: &'static str,
}

impl InsertDropsCtx {
    pub fn new(variables_count: usize) -> Self {
        Self {
            variables_count,
            depth: 1,
            message: "Nothing",
        }
    }

    pub fn deeper(&self, message: &'static str) -> Self {
        Self {
            variables_count: self.variables_count,
            depth: self.depth + 1,
            message,
        }
    }

    pub fn log_declare(&self, variable: VariableStorageKey) {
        lifetime_log!(
            "[DEPTH {} {} DECLARE] Variable {}",
            self.depth,
            &self.message,
            variable.index
        );
    }

    pub fn log_use(&self, variable: VariableStorageKey) {
        lifetime_log!(
            "[DEPTH {} {} USE] Variable {}",
            self.depth,
            &self.message,
            variable.index
        );
    }

    pub fn log_drop(&self, variable_index: usize, drop_after: usize) {
        lifetime_log!(
            "[DEPTH {} {}] dropping variable {} after stmt {}",
            self.depth,
            self.message,
            variable_index,
            drop_after
        );
    }
}

pub fn insert_drops(function: &mut Function) {
    // Search through statements top to bottom, and record which statement each variable storage
    // key is last used by

    insert_drops_for_stmts(
        InsertDropsCtx::new(function.variables.count()),
        &mut function.stmts,
    );

    let mut active_set = ActiveSet::new(function.variables.count());
    integrate_active_set_for_stmts(&mut function.stmts, &mut active_set);
}

fn insert_drops_for_stmts(ctx: InsertDropsCtx, stmts: &mut Vec<Stmt>) -> VariableUsageSet {
    let mut last_use_in_this_scope = vec![0 as usize; ctx.variables_count];
    let mut scope = VariableUsageSet::new(ctx.variables_count.try_into().unwrap());

    for (stmt_index, stmt) in stmts.iter_mut().enumerate() {
        let mentioned = match &mut stmt.kind {
            StmtKind::Return(expr, _) => {
                if let Some(expr) = expr {
                    insert_drops_for_expr(ctx.clone(), expr)
                } else {
                    VariableUsageSet::new(ctx.variables_count)
                }
            }
            StmtKind::Expr(expr) => insert_drops_for_expr(ctx.clone(), &mut expr.expr),
            StmtKind::Declaration(declaration) => {
                ctx.log_declare(declaration.key);
                scope.declare(declaration.key);
                last_use_in_this_scope[declaration.key.index] = stmt_index;

                if let Some(expr) = &mut declaration.value {
                    insert_drops_for_expr(ctx.clone(), expr)
                } else {
                    VariableUsageSet::new(ctx.variables_count)
                }
            }
            StmtKind::Assignment(assignment) => {
                let mut mentioned = insert_drops_for_expr(ctx.clone(), &mut assignment.value);
                mentioned.union_with(&insert_drops_for_destination(
                    ctx.clone(),
                    &mut assignment.destination,
                ));
                mentioned
            }
        };

        scope.union_with(&mentioned);

        for (mention_index, did_declare) in mentioned.iter_declared().enumerate() {
            if did_declare {
                last_use_in_this_scope[mention_index] = stmt_index;
            }
        }

        for (mention_index, did_mention) in mentioned.iter_used().enumerate() {
            if did_mention {
                last_use_in_this_scope[mention_index] = stmt_index;
            }
        }
    }

    // For each variable declared in reverse declaration order,
    for (variable_index, did_declare) in scope.iter_declared().enumerate().rev() {
        if !did_declare {
            continue;
        }

        let drop_after = last_use_in_this_scope[variable_index];

        ctx.log_drop(variable_index, drop_after);

        stmts[drop_after].drops.push(VariableStorageKey {
            index: variable_index,
        });
    }

    scope
}

fn insert_drops_for_expr(ctx: InsertDropsCtx, expr: &mut Expr) -> VariableUsageSet {
    let mut mini_scope = VariableUsageSet::new(ctx.variables_count);

    match &mut expr.kind {
        ExprKind::Variable(variable) => {
            ctx.log_use(variable.key);
            mini_scope.mark_used(variable.key);
        }
        ExprKind::GlobalVariable(..) => (),
        ExprKind::BooleanLiteral(..)
        | ExprKind::IntegerLiteral(..)
        | ExprKind::Integer { .. }
        | ExprKind::Float(..)
        | ExprKind::String(..)
        | ExprKind::NullTerminatedString(..) => (),
        ExprKind::Call(call) => {
            for argument in call.arguments.iter_mut() {
                mini_scope.union_with(&insert_drops_for_expr(ctx.clone(), argument));
            }
        }
        ExprKind::DeclareAssign(declare_assign) => {
            mini_scope.union_with(&insert_drops_for_expr(
                ctx.clone(),
                &mut declare_assign.value,
            ));

            ctx.log_declare(declare_assign.key);
            mini_scope.declare(declare_assign.key);
        }
        ExprKind::BasicBinaryOperation(operation) => {
            mini_scope.union_with(&insert_drops_for_expr(
                ctx.clone(),
                &mut operation.left.expr,
            ));
            mini_scope.union_with(&insert_drops_for_expr(
                ctx.clone(),
                &mut operation.right.expr,
            ));
        }
        ExprKind::ShortCircuitingBinaryOperation(operation) => {
            mini_scope.union_with(&insert_drops_for_expr(
                ctx.clone(),
                &mut operation.left.expr,
            ));

            let hidden_scope =
                &insert_drops_for_expr(ctx.deeper("short circuitable"), &mut operation.right.expr);

            // Variables declared in the potentially skipped section need to be dropped
            // in the case that the section is executed
            for (variable_index, did_declare) in hidden_scope.iter_declared().enumerate() {
                if did_declare {
                    operation.drops.push(VariableStorageKey {
                        index: variable_index,
                    });

                    lifetime_log!(
                        "[DEPTH {} {}] dropping variable {} after short circuit op",
                        ctx.depth,
                        ctx.message,
                        variable_index
                    );
                }
            }

            mini_scope.used.or(&hidden_scope.used);
        }
        ExprKind::IntegerExtend(value, _) => {
            mini_scope.union_with(&insert_drops_for_expr(ctx, value));
        }
        ExprKind::FloatExtend(value, _) => {
            mini_scope.union_with(&insert_drops_for_expr(ctx, value));
        }
        ExprKind::Member { subject, .. } => {
            mini_scope.union_with(&insert_drops_for_destination(ctx, subject));
        }
        ExprKind::StructureLiteral { .. } => (),
        ExprKind::UnaryOperation(operation) => {
            mini_scope.union_with(&insert_drops_for_expr(
                ctx.clone(),
                &mut operation.inner.expr,
            ));
        }
        ExprKind::Conditional(conditional) => {
            if let Some(branch) = conditional.branches.first_mut() {
                let condition_scope =
                    insert_drops_for_expr(ctx.clone(), &mut branch.condition.expr);

                mini_scope.union_with(&condition_scope);
            }

            lifetime_log!("warning: declaring variables in conditions of non-first branches is not properly handled yet");

            for branch in conditional.branches.iter_mut() {
                let inner_scope =
                    insert_drops_for_stmts(ctx.deeper("if branch"), &mut branch.block.stmts);
                mini_scope.used.or(&inner_scope.used);
            }

            if let Some(otherwise) = &mut conditional.otherwise {
                let inner_scope =
                    insert_drops_for_stmts(ctx.deeper("otherwise branch"), &mut otherwise.stmts);
                mini_scope.used.or(&inner_scope.used);
            }
        }
        ExprKind::While(while_loop) => {
            mini_scope.union_with(&insert_drops_for_expr(
                ctx.clone(),
                &mut while_loop.condition,
            ));

            let inner_scope =
                insert_drops_for_stmts(ctx.deeper("while body"), &mut while_loop.block.stmts);
            mini_scope.used.or(&inner_scope.used);
        }
        ExprKind::ArrayAccess(array_access) => {
            mini_scope.union_with(&insert_drops_for_expr(
                ctx.clone(),
                &mut array_access.subject,
            ));
            mini_scope.union_with(&insert_drops_for_expr(ctx.clone(), &mut array_access.index));
        }
        ExprKind::EnumMemberLiteral(_enum_member_literal) => (),
        ExprKind::ResolvedNamedExpression(_name, resolved_expr) => {
            mini_scope = insert_drops_for_expr(ctx, resolved_expr);
        }
    }

    mini_scope
}

fn insert_drops_for_destination(
    ctx: InsertDropsCtx,
    destination: &mut Destination,
) -> VariableUsageSet {
    let mut mini_scope = VariableUsageSet::new(ctx.variables_count);

    match &mut destination.kind {
        DestinationKind::Variable(variable) => {
            ctx.log_use(variable.key);
            mini_scope.mark_used(variable.key);
        }
        DestinationKind::GlobalVariable(_) => (),
        DestinationKind::Member { subject, .. } => {
            mini_scope.union_with(&insert_drops_for_destination(ctx, subject));
        }
        DestinationKind::ArrayAccess(array_access) => {
            mini_scope.union_with(&insert_drops_for_expr(
                ctx.clone(),
                &mut array_access.subject,
            ));
            mini_scope.union_with(&insert_drops_for_expr(ctx.clone(), &mut array_access.index));
        }
    }

    mini_scope
}

#[derive(Clone, Debug)]
struct ActiveSet {
    pub active: BitVec,
}

impl ActiveSet {
    pub fn new(count: usize) -> Self {
        Self {
            active: BitVec::from_elem(count, false),
        }
    }

    pub fn activate(&mut self, variable: VariableStorageKey) {
        self.active.set(variable.index, true);
    }

    pub fn deactivate_drops(&mut self, drops: &Drops) {
        for variable in drops.iter() {
            self.active.set(variable.index, false);
        }
    }
}

fn integrate_active_set_for_stmts(stmts: &mut Vec<Stmt>, parent_active_set: &mut ActiveSet) {
    let mut active_set = parent_active_set.clone();

    for (_stmt_i, stmt) in stmts.iter_mut().enumerate() {
        match &mut stmt.kind {
            StmtKind::Return(value, drops) => {
                stmt.drops.drops.clear();

                if let Some(value) = value {
                    integrate_active_set_for_expr(value, &mut active_set);
                }

                for (variable_index, active) in active_set.active.iter().enumerate().rev() {
                    if active {
                        /*
                        println!(
                            "[RETURN DROP] variable {} during return stmt {}",
                            variable_index, _stmt_i,
                        );
                        */

                        drops.push(VariableStorageKey {
                            index: variable_index,
                        });
                    }
                }
            }
            StmtKind::Expr(expr) => {
                integrate_active_set_for_expr(&mut expr.expr, &mut active_set);
            }
            StmtKind::Declaration(declaration) => {
                if let Some(value) = &mut declaration.value {
                    integrate_active_set_for_expr(value, &mut active_set);
                }

                active_set.activate(declaration.key);
            }
            StmtKind::Assignment(assignment) => {
                integrate_active_set_for_expr(&mut assignment.value, &mut active_set);
            }
        }

        active_set.deactivate_drops(&stmt.drops);
    }
}

fn integrate_active_set_for_expr(expr: &mut Expr, active_set: &mut ActiveSet) {
    match &mut expr.kind {
        ExprKind::Variable(_)
        | ExprKind::GlobalVariable(_)
        | ExprKind::BooleanLiteral(_)
        | ExprKind::IntegerLiteral(_)
        | ExprKind::Integer { .. }
        | ExprKind::Float(..)
        | ExprKind::String(_)
        | ExprKind::NullTerminatedString(_)
        | ExprKind::EnumMemberLiteral(_) => (),
        ExprKind::Call(call) => {
            for argument in call.arguments.iter_mut() {
                integrate_active_set_for_expr(argument, active_set);
            }
        }
        ExprKind::DeclareAssign(declare_assign) => {
            integrate_active_set_for_expr(&mut declare_assign.value, active_set);
            active_set.activate(declare_assign.key);
        }
        ExprKind::BasicBinaryOperation(operation) => {
            integrate_active_set_for_expr(&mut operation.left.expr, active_set);
            integrate_active_set_for_expr(&mut operation.right.expr, active_set);
        }
        ExprKind::ShortCircuitingBinaryOperation(operation) => {
            integrate_active_set_for_expr(&mut operation.left.expr, active_set);
            integrate_active_set_for_expr(&mut operation.right.expr, active_set);
            active_set.deactivate_drops(&operation.drops);
        }
        ExprKind::IntegerExtend(..) | ExprKind::FloatExtend(..) => (),
        ExprKind::Member { subject, .. } => {
            integrate_active_set_for_destination(subject, active_set);
        }
        ExprKind::StructureLiteral { fields, .. } => {
            for (_name, (expr, _index)) in fields.iter_mut() {
                integrate_active_set_for_expr(expr, active_set);
            }
        }
        ExprKind::UnaryOperation(operation) => {
            integrate_active_set_for_expr(&mut operation.inner.expr, active_set);
        }
        ExprKind::Conditional(conditional) => {
            for branch in conditional.branches.iter_mut() {
                integrate_active_set_for_expr(&mut branch.condition.expr, active_set);
                integrate_active_set_for_stmts(&mut branch.block.stmts, active_set);
            }

            if let Some(otherwise) = &mut conditional.otherwise {
                integrate_active_set_for_stmts(&mut otherwise.stmts, active_set);
            }
        }
        ExprKind::While(while_loop) => {
            integrate_active_set_for_expr(&mut while_loop.condition, active_set);
            integrate_active_set_for_stmts(&mut while_loop.block.stmts, active_set);
        }
        ExprKind::ArrayAccess(array_access) => {
            integrate_active_set_for_expr(&mut array_access.subject, active_set);
            integrate_active_set_for_expr(&mut array_access.index, active_set);
        }
        ExprKind::ResolvedNamedExpression(_name, resolved_expr) => {
            integrate_active_set_for_expr(resolved_expr.as_mut(), active_set);
        }
    }
}

fn integrate_active_set_for_destination(destination: &mut Destination, active_set: &mut ActiveSet) {
    match &mut destination.kind {
        DestinationKind::Variable(_) | DestinationKind::GlobalVariable(_) => (),
        DestinationKind::Member { subject, .. } => {
            integrate_active_set_for_destination(subject, active_set);
        }
        DestinationKind::ArrayAccess(array_access) => {
            integrate_active_set_for_expr(&mut array_access.subject, active_set);
            integrate_active_set_for_expr(&mut array_access.index, active_set);
        }
    }
}
