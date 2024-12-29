use super::{builder::Builder, datatype::lower_type, error::LowerError, expr::lower_expr};
use crate::{
    asg::{self, Asg, Cast, CastFrom},
    data_units::ByteUnits,
    ir::{self, Value},
    target::Target,
};

pub fn integer_like_type_size(target: &Target, ty: &asg::Type) -> Option<ByteUnits> {
    match &ty.kind {
        asg::TypeKind::Boolean => Some(target.bool_layout().width),
        asg::TypeKind::Integer(bits, _) => Some(bits.bytes()),
        asg::TypeKind::CInteger(c_integer, _) => Some(c_integer.bytes(target)),
        _ => None,
    }
}

pub fn integer_truncate(
    builder: &mut Builder,
    ir_module: &ir::Module,
    function: &asg::Function,
    asg: &Asg,
    cast: &Cast,
) -> Result<Value, LowerError> {
    let value = lower_expr(builder, ir_module, &cast.value, function, asg)?;
    let ir_type = lower_type(ir_module, &builder.unpoly(&cast.target_type)?, asg)?;
    Ok(builder.push(ir::Instruction::Truncate(value, ir_type)))
}

pub fn integer_extend(
    builder: &mut Builder,
    ir_module: &ir::Module,
    function: &asg::Function,
    asg: &Asg,
    cast_from: &CastFrom,
) -> Result<Value, LowerError> {
    let value = lower_expr(builder, ir_module, &cast_from.cast.value, function, asg)?;

    let ir_type = lower_type(
        ir_module,
        &builder.unpoly(&cast_from.cast.target_type)?,
        asg,
    )?;

    Ok(builder.push(
        match cast_from
            .from_type
            .kind
            .sign(Some(&ir_module.target))
            .expect("integer extend result type to be an integer type")
        {
            asg::IntegerSign::Signed => ir::Instruction::SignExtend(value, ir_type),
            asg::IntegerSign::Unsigned => ir::Instruction::ZeroExtend(value, ir_type),
        },
    ))
}

pub fn integer_cast(
    builder: &mut Builder,
    ir_module: &ir::Module,
    function: &asg::Function,
    asg: &Asg,
    cast_from: &CastFrom,
) -> Result<Value, LowerError> {
    let from_size = integer_like_type_size(&ir_module.target, &cast_from.from_type)
        .expect("from type to be an integer");
    let to_size = integer_like_type_size(&ir_module.target, &cast_from.cast.target_type)
        .expect("to type to be an integer");

    if from_size < to_size {
        integer_extend(builder, ir_module, function, asg, &cast_from)
    } else if from_size > to_size {
        integer_truncate(builder, ir_module, function, asg, &cast_from.cast)
    } else {
        Ok(lower_expr(
            builder,
            ir_module,
            &cast_from.cast.value,
            function,
            asg,
        )?)
    }
}
