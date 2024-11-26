use super::lower_expr;
use crate::{
    ir::{self, Literal, Value},
    lower::{builder::Builder, error::LowerError},
    resolved,
};

#[derive(Debug)]
pub struct BinaryShortCircuit {
    pub left: Value,
    pub right: Value,
    pub left_done_block_id: usize,
    pub right_done_block_id: usize,
    pub evaluate_right_block_id: usize,
}

pub fn lower_short_circuiting_binary_operation(
    builder: &mut Builder,
    ir_module: &ir::Module,
    operation: &resolved::ShortCircuitingBinaryOperation,
    function: &resolved::Function,
    resolved_ast: &resolved::Ast,
) -> Result<Value, LowerError> {
    let short_circuit =
        lower_pre_short_circuit(builder, ir_module, operation, function, resolved_ast)?;
    let merge_block_id = builder.new_block();
    builder.continues_to(merge_block_id);
    builder.use_block(short_circuit.left_done_block_id);

    let (conditional_break, early_result) = match operation.operator {
        resolved::ShortCircuitingBinaryOperator::And => (
            ir::ConditionalBreak {
                true_basicblock_id: short_circuit.evaluate_right_block_id,
                false_basicblock_id: merge_block_id,
            },
            false,
        ),
        resolved::ShortCircuitingBinaryOperator::Or => (
            ir::ConditionalBreak {
                true_basicblock_id: merge_block_id,
                false_basicblock_id: short_circuit.evaluate_right_block_id,
            },
            true,
        ),
    };

    builder.push(ir::Instruction::ConditionalBreak(
        short_circuit.left,
        conditional_break,
    ));
    builder.use_block(merge_block_id);

    Ok(builder.push(ir::Instruction::Phi(ir::Phi {
        ir_type: ir::Type::Boolean,
        incoming: vec![
            ir::PhiIncoming {
                basicblock_id: short_circuit.left_done_block_id,
                value: ir::Value::Literal(Literal::Boolean(early_result)),
            },
            ir::PhiIncoming {
                basicblock_id: short_circuit.right_done_block_id,
                value: short_circuit.right,
            },
        ],
    })))
}

pub fn lower_pre_short_circuit(
    builder: &mut Builder,
    ir_module: &ir::Module,
    operation: &resolved::ShortCircuitingBinaryOperation,
    function: &resolved::Function,
    resolved_ast: &resolved::Ast,
) -> Result<BinaryShortCircuit, LowerError> {
    let left = lower_expr(
        builder,
        ir_module,
        &operation.left.expr,
        function,
        resolved_ast,
    )?;

    let left_done_block_id = builder.current_block_id();
    let evaluate_right_block_id = builder.new_block();
    builder.use_block(evaluate_right_block_id);

    let right = lower_expr(
        builder,
        ir_module,
        &operation.right.expr,
        function,
        resolved_ast,
    )?;
    let right_done_block_id = builder.current_block_id();

    Ok(BinaryShortCircuit {
        left,
        right,
        left_done_block_id,
        right_done_block_id,
        evaluate_right_block_id,
    })
}
