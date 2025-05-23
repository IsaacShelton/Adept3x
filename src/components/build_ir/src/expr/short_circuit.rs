use super::lower_expr;
use crate::{error::LowerError, func_builder::FuncBuilder};

#[derive(Debug)]
pub struct BinaryShortCircuit {
    pub left: ir::Value,
    pub right: ir::Value,
    pub left_done_block_id: usize,
    pub right_done_block_id: usize,
    pub evaluate_right_block_id: usize,
}

pub fn lower_short_circuiting_binary_operation(
    builder: &mut FuncBuilder,
    operation: &asg::ShortCircuitingBinaryOperation,
) -> Result<ir::Value, LowerError> {
    let short_circuit = lower_pre_short_circuit(builder, operation)?;
    let merge_block_id = builder.new_block();
    builder.continues_to(merge_block_id);
    builder.use_block(short_circuit.left_done_block_id);

    let (conditional_break, early_result) = match operation.operator {
        asg::ShortCircuitingBinaryOperator::And => (
            ir::ConditionalBreak {
                true_basicblock_id: short_circuit.evaluate_right_block_id,
                false_basicblock_id: merge_block_id,
            },
            false,
        ),
        asg::ShortCircuitingBinaryOperator::Or => (
            ir::ConditionalBreak {
                true_basicblock_id: merge_block_id,
                false_basicblock_id: short_circuit.evaluate_right_block_id,
            },
            true,
        ),
    };

    builder.push(ir::Instr::ConditionalBreak(
        short_circuit.left,
        conditional_break,
    ));
    builder.use_block(merge_block_id);

    Ok(builder.push(ir::Instr::Phi(ir::Phi {
        ir_type: ir::Type::Bool,
        incoming: vec![
            ir::PhiIncoming {
                basicblock_id: short_circuit.left_done_block_id,
                value: ir::Value::Literal(ir::Literal::Boolean(early_result)),
            },
            ir::PhiIncoming {
                basicblock_id: short_circuit.right_done_block_id,
                value: short_circuit.right,
            },
        ],
    })))
}

pub fn lower_pre_short_circuit(
    builder: &mut FuncBuilder,
    operation: &asg::ShortCircuitingBinaryOperation,
) -> Result<BinaryShortCircuit, LowerError> {
    let left = lower_expr(builder, &operation.left.expr)?;

    let left_done_block_id = builder.current_block_id();
    let evaluate_right_block_id = builder.new_block();
    builder.use_block(evaluate_right_block_id);

    let right = builder.lower_expr(&operation.right.expr)?;
    let right_done_block_id = builder.current_block_id();

    Ok(BinaryShortCircuit {
        left,
        right,
        left_done_block_id,
        right_done_block_id,
        evaluate_right_block_id,
    })
}
