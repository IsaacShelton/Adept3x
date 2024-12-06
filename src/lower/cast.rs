use super::{builder::Builder, datatype::lower_type, error::LowerError, expr::lower_expr};
use crate::{
    data_units::ByteUnits,
    ir::{self, Value},
    resolved::{self, Cast, CastFrom},
    target::Target,
};

pub fn integer_like_type_size(target: &Target, ty: &resolved::Type) -> Option<ByteUnits> {
    match &ty.kind {
        resolved::TypeKind::Boolean => Some(target.bool_layout().width),
        resolved::TypeKind::Integer(bits, _) => Some(bits.bytes()),
        resolved::TypeKind::CInteger(c_integer, _) => Some(c_integer.bytes(target)),
        _ => None,
    }
}

pub fn integer_truncate(
    builder: &mut Builder,
    ir_module: &ir::Module,
    function: &resolved::Function,
    resolved_ast: &resolved::Ast,
    cast: &Cast,
) -> Result<Value, LowerError> {
    let value = lower_expr(builder, ir_module, &cast.value, function, resolved_ast)?;
    let ir_type = lower_type(ir_module, &builder.unpoly(&cast.target_type)?, resolved_ast)?;
    Ok(builder.push(ir::Instruction::Truncate(value, ir_type)))
}

pub fn integer_extend(
    builder: &mut Builder,
    ir_module: &ir::Module,
    function: &resolved::Function,
    resolved_ast: &resolved::Ast,
    cast_from: &CastFrom,
) -> Result<Value, LowerError> {
    let value = lower_expr(
        builder,
        ir_module,
        &cast_from.cast.value,
        function,
        resolved_ast,
    )?;

    let ir_type = lower_type(
        ir_module,
        &builder.unpoly(&cast_from.cast.target_type)?,
        resolved_ast,
    )?;

    Ok(builder.push(
        match cast_from
            .from_type
            .kind
            .sign(Some(&ir_module.target))
            .expect("integer extend result type to be an integer type")
        {
            resolved::IntegerSign::Signed => ir::Instruction::SignExtend(value, ir_type),
            resolved::IntegerSign::Unsigned => ir::Instruction::ZeroExtend(value, ir_type),
        },
    ))
}

pub fn integer_cast(
    builder: &mut Builder,
    ir_module: &ir::Module,
    function: &resolved::Function,
    resolved_ast: &resolved::Ast,
    cast_from: &CastFrom,
) -> Result<Value, LowerError> {
    let from_size = integer_like_type_size(&ir_module.target, &cast_from.from_type)
        .expect("from type to be an integer");
    let to_size = integer_like_type_size(&ir_module.target, &cast_from.cast.target_type)
        .expect("to type to be an integer");

    if from_size < to_size {
        integer_extend(builder, ir_module, function, resolved_ast, &cast_from)
    } else if from_size > to_size {
        integer_truncate(builder, ir_module, function, resolved_ast, &cast_from.cast)
    } else {
        Ok(lower_expr(
            builder,
            ir_module,
            &cast_from.cast.value,
            function,
            resolved_ast,
        )?)
    }
}
