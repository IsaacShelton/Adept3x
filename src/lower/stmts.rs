use super::{
    builder::Builder,
    datatype::lower_type,
    error::LowerError,
    expr::{lower_basic_binary_operation, lower_destination, lower_expr},
};
use crate::{
    asg::{self, Asg, StmtKind},
    ir::{self, Literal, Value, ValueReference},
    tag::Tag,
};

pub fn lower_stmts(
    builder: &mut Builder,
    ir_module: &ir::Module,
    stmts: &[asg::Stmt],
    function: &asg::Function,
    asg: &Asg,
) -> Result<Value, LowerError> {
    let mut result = Value::Literal(Literal::Void);

    for stmt in stmts.iter() {
        result = match &stmt.kind {
            StmtKind::Return(expr) => {
                let instruction = ir::Instruction::Return(if let Some(expr) = expr {
                    Some(lower_expr(builder, ir_module, expr, function, asg)?)
                } else if function.tag == Some(Tag::Main) {
                    Some(ir::Value::Literal(Literal::Signed32(0)))
                } else {
                    None
                });

                builder.push(instruction);
                Value::Literal(Literal::Void)
            }
            StmtKind::Expr(expr) => lower_expr(builder, ir_module, &expr.expr, function, asg)?,
            StmtKind::Declaration(declaration) => {
                let destination = Value::Reference(ValueReference {
                    basicblock_id: 0,
                    instruction_id: declaration.key.index,
                });

                if let Some(value) = &declaration.value {
                    let source = lower_expr(builder, ir_module, value, function, asg)?;

                    builder.push(ir::Instruction::Store(ir::Store {
                        new_value: source,
                        destination,
                    }));
                }

                Value::Literal(Literal::Void)
            }
            StmtKind::Assignment(assignment) => {
                let destination =
                    lower_destination(builder, ir_module, &assignment.destination, function, asg)?;

                let new_value = lower_expr(builder, ir_module, &assignment.value, function, asg)?;

                let new_value = if let Some(operator) = &assignment.operator {
                    let destination_type = lower_type(
                        ir_module,
                        &builder.unpoly(&assignment.destination.resolved_type)?,
                        asg,
                    )?;

                    let existing_value = builder.push(ir::Instruction::Load((
                        destination.clone(),
                        destination_type,
                    )));

                    lower_basic_binary_operation(
                        builder,
                        ir_module,
                        operator,
                        ir::BinaryOperands::new(existing_value, new_value),
                    )?
                } else {
                    new_value
                };

                builder.push(ir::Instruction::Store(ir::Store {
                    new_value,
                    destination,
                }));

                Value::Literal(Literal::Void)
            }
        };
    }

    Ok(result)
}
