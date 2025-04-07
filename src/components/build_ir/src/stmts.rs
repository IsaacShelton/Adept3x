use super::{error::LowerError, expr::lower_basic_binary_operation, func_builder::FuncBuilder};
use asg::StmtKind;
use attributes::Tag;
use ir::{Literal, Value, ValueReference};

pub fn lower_stmts(builder: &mut FuncBuilder, stmts: &[asg::Stmt]) -> Result<Value, LowerError> {
    let mut result = Value::Literal(Literal::Void);

    for stmt in stmts.iter() {
        result = match &stmt.kind {
            StmtKind::Return(expr) => {
                let instruction = ir::Instr::Return(if let Some(expr) = expr {
                    Some(builder.lower_expr(expr)?)
                } else if builder.asg_func().tag == Some(Tag::Main) {
                    Some(ir::Value::Literal(Literal::Signed32(0)))
                } else {
                    None
                });

                builder.push(instruction);
                return Ok(Value::Literal(Literal::Void));
            }
            StmtKind::Expr(expr) => builder.lower_expr(&expr.expr)?,
            StmtKind::Declaration(declaration) => {
                let destination = Value::Reference(ValueReference {
                    basicblock_id: 0,
                    instruction_id: declaration.key.index,
                });

                if let Some(value) = &declaration.value {
                    let new_value = builder.lower_expr(value)?;

                    builder.push(ir::Instr::Store(ir::Store {
                        new_value,
                        destination,
                    }));
                }

                Value::Literal(Literal::Void)
            }
            StmtKind::Assignment(assignment) => {
                let destination = builder.lower_destination(&assignment.destination)?;
                let new_value = builder.lower_expr(&assignment.value)?;

                let new_value = if let Some(operator) = &assignment.operator {
                    let destination_type = builder.lower_type(&assignment.destination.ty)?;

                    let existing_value =
                        builder.push(ir::Instr::Load((destination.clone(), destination_type)));

                    lower_basic_binary_operation(
                        builder,
                        operator,
                        ir::BinaryOperands::new(existing_value, new_value),
                    )?
                } else {
                    new_value
                };

                builder.push(ir::Instr::Store(ir::Store {
                    new_value,
                    destination,
                }));

                Value::Literal(Literal::Void)
            }
        };
    }

    Ok(result)
}

pub fn lower_stmts_with_break_and_continue(
    builder: &mut FuncBuilder,
    stmts: &[asg::Stmt],
    break_basicblock_id: Option<usize>,
    continue_basicblock_id: Option<usize>,
) -> Result<Value, LowerError> {
    let prev_break_basicblock_id = builder.break_basicblock_id;
    let prev_continue_basicblock_id = builder.continue_basicblock_id;

    builder.break_basicblock_id = break_basicblock_id.or(builder.break_basicblock_id);
    builder.continue_basicblock_id = continue_basicblock_id.or(builder.continue_basicblock_id);

    let result = builder.lower_stmts(stmts);

    builder.break_basicblock_id = prev_break_basicblock_id;
    builder.continue_basicblock_id = prev_continue_basicblock_id;
    result
}
