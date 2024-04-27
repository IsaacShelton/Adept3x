use crate::resolved::{self, Destination, VariableStorage, VariableStorageKey};
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

    pub fn deactivate(&mut self, variable: VariableStorageKey) {
        self.active.set(variable.index, false);
    }

    pub fn deactivate_scope(&mut self, declared_in_this_scope: &BitVec) {
        self.active.difference(declared_in_this_scope);
    }
}

pub fn insert_drops(function: &mut resolved::Function) {
    // Search through statements top to bottom, and record which statement each variable storage
    // key is last used by

    let mut active_set = ActiveSet::new(function.variables.count());
    insert_drops_for_stmts(&mut function.stmts, &function.variables, &mut active_set);
}

fn insert_drops_for_stmts(
    stmts: &mut Vec<resolved::Stmt>,
    variables: &VariableStorage,
    active_set: &mut ActiveSet,
) -> VariableUsageSet {
    let count = variables.count();
    let mut last_use_in_this_scope = vec![0 as usize; count];
    let mut scope = VariableUsageSet::new(count.try_into().unwrap());

    for (i, stmt) in stmts.iter_mut().enumerate() {
        let mentioned = match &mut stmt.kind {
            resolved::StmtKind::Return(expr, _drops) => {
                eprintln!("warning: need to append drops when returning");

                if let Some(expr) = expr {
                    insert_drops_for_expr(count, expr, active_set)
                } else {
                    VariableUsageSet::new(count)
                }
            }
            resolved::StmtKind::Expr(expr) => {
                insert_drops_for_expr(count, &mut expr.expr, active_set)
            }
            resolved::StmtKind::Declaration(declaration) => {
                scope.declare(declaration.key);
                active_set.activate(declaration.key);
                last_use_in_this_scope[declaration.key.index] = i;

                if let Some(expr) = &mut declaration.value {
                    insert_drops_for_expr(count, expr, active_set)
                } else {
                    VariableUsageSet::new(count)
                }
            }
            resolved::StmtKind::Assignment(assignment) => {
                let mut mentioned = insert_drops_for_expr(count, &mut assignment.value, active_set);
                mentioned.union_with(&insert_drops_for_destination(
                    count,
                    &mut assignment.destination,
                    active_set,
                ));
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

    // For each variable declared in reverse declaration order,
    for (variable_index, did_declare) in scope.iter_declared().rev().enumerate() {
        if !did_declare {
            continue;
        }

        let drop_after = last_use_in_this_scope[variable_index];

        println!(
            "Dropping variable {} after statement {}",
            variable_index, drop_after
        );

        println!("(warning, returning does not properly drop yet)",);

        stmts[drop_after].drops.push(VariableStorageKey {
            index: variable_index,
        });

        println!("{:?}", &stmts[drop_after].drops);
    }

    active_set.deactivate_scope(&scope.declared);
    scope
}

fn insert_drops_for_expr(
    variable_count: usize,
    expr: &mut resolved::Expr,
    active_set: &mut ActiveSet,
) -> VariableUsageSet {
    let mut mini_scope = VariableUsageSet::new(variable_count);

    match &mut expr.kind {
        resolved::ExprKind::Variable(variable) => {
            mini_scope.mark_used(variable.key);
        }
        resolved::ExprKind::GlobalVariable(..) => (),
        resolved::ExprKind::BooleanLiteral(..)
        | resolved::ExprKind::IntegerLiteral(..)
        | resolved::ExprKind::Integer { .. }
        | resolved::ExprKind::Float(..)
        | resolved::ExprKind::String(..)
        | resolved::ExprKind::NullTerminatedString(..) => (),
        resolved::ExprKind::Call(call) => {
            for argument in call.arguments.iter_mut() {
                mini_scope.union_with(&insert_drops_for_expr(variable_count, argument, active_set));
            }
        }
        resolved::ExprKind::DeclareAssign(declare_assign) => {
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut declare_assign.value,
                active_set,
            ));

            mini_scope.declare(declare_assign.key);
            active_set.activate(declare_assign.key);
        }
        resolved::ExprKind::BasicBinaryOperation(operation) => {
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut operation.left.expr,
                active_set,
            ));
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut operation.right.expr,
                active_set,
            ));
        }
        resolved::ExprKind::ShortCircuitingBinaryOperation(operation) => {
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut operation.left.expr,
                active_set,
            ));

            let additional =
                &insert_drops_for_expr(variable_count, &mut operation.right.expr, active_set);

            // Variables declared in the potentially skipped section need to be dropped
            // in the case that the section is executed
            for (variable_index, did_declare) in additional.iter_declared().enumerate() {
                if did_declare {
                    println!("Inside-statement dropping variable {}", variable_index);

                    operation.drops.push(VariableStorageKey {
                        index: variable_index,
                    });

                    println!("{:?}", &operation.drops);
                }
            }

            mini_scope.used.or(&additional.used);
        }
        resolved::ExprKind::IntegerExtend(value, _) => {
            mini_scope.union_with(&insert_drops_for_expr(variable_count, value, active_set));
        }
        resolved::ExprKind::FloatExtend(value, _) => {
            mini_scope.union_with(&insert_drops_for_expr(variable_count, value, active_set));
        }
        resolved::ExprKind::Member { .. } => (),
        resolved::ExprKind::StructureLiteral { .. } => (),
        resolved::ExprKind::UnaryOperation(operation) => {
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut operation.inner.expr,
                active_set,
            ));
        }
        resolved::ExprKind::Conditional(conditional) => {
            if let Some(branch) = conditional.branches.first_mut() {
                mini_scope.union_with(&insert_drops_for_expr(
                    variable_count,
                    &mut branch.condition.expr,
                    active_set,
                ));
            }
        }
        resolved::ExprKind::While(while_loop) => {
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut while_loop.condition,
                active_set,
            ));
        }
        resolved::ExprKind::ArrayAccess(array_access) => {
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut array_access.subject,
                active_set,
            ));
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut array_access.index,
                active_set,
            ));
        }
    }

    mini_scope
}

fn insert_drops_for_destination(
    variable_count: usize,
    destination: &mut Destination,
    active_set: &mut ActiveSet,
) -> VariableUsageSet {
    let mut mini_scope = VariableUsageSet::new(variable_count);

    match &mut destination.kind {
        resolved::DestinationKind::Variable(variable) => {
            mini_scope.mark_used(variable.key);
        }
        resolved::DestinationKind::GlobalVariable(_) => (),
        resolved::DestinationKind::Member { subject, .. } => {
            mini_scope.union_with(&insert_drops_for_destination(
                variable_count,
                subject,
                active_set,
            ));
        }
        resolved::DestinationKind::ArrayAccess(array_access) => {
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut array_access.subject,
                active_set,
            ));
            mini_scope.union_with(&insert_drops_for_expr(
                variable_count,
                &mut array_access.index,
                active_set,
            ));
        }
    }

    mini_scope
}
