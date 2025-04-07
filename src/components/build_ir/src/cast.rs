use super::{error::LowerError, expr::lower_expr, func_builder::FuncBuilder};
use asg::{Cast, CastFrom};
use data_units::ByteUnits;
use ir::Value;
use primitives::IntegerSign;
use target::Target;
use target_layout::TargetLayout;

pub fn integer_like_type_size(target: &Target, ty: &asg::Type) -> Option<ByteUnits> {
    match &ty.kind {
        asg::TypeKind::Boolean => Some(target.bool_layout().width),
        asg::TypeKind::Integer(bits, _) => Some(bits.bytes()),
        asg::TypeKind::CInteger(c_integer, _) => Some(target.c_integer_bytes(*c_integer)),
        asg::TypeKind::SizeInteger(_) => Some(target.size_layout().width),
        _ => None,
    }
}

pub fn integer_truncate(builder: &mut FuncBuilder, cast: &Cast) -> Result<Value, LowerError> {
    let value = builder.lower_expr(&cast.value)?;
    let ir_type = builder.lower_type(&cast.target_type)?;
    Ok(builder.push(ir::Instr::Truncate(value, ir_type)))
}

pub fn integer_extend(
    builder: &mut FuncBuilder,
    cast_from: &CastFrom,
) -> Result<Value, LowerError> {
    let value = builder.lower_expr(&cast_from.cast.value)?;
    let ir_type = builder.lower_type(&cast_from.cast.target_type)?;

    Ok(builder.push(
        match cast_from
            .from_type
            .kind
            .sign(Some(builder.target()))
            .expect("integer extend result type to be an integer type")
        {
            IntegerSign::Signed => ir::Instr::SignExtend(value, ir_type),
            IntegerSign::Unsigned => ir::Instr::ZeroExtend(value, ir_type),
        },
    ))
}

pub fn integer_cast(builder: &mut FuncBuilder, cast_from: &CastFrom) -> Result<Value, LowerError> {
    let from_size = integer_like_type_size(builder.target(), &cast_from.from_type)
        .expect("from type to be an integer");
    let to_size = integer_like_type_size(builder.target(), &cast_from.cast.target_type)
        .expect("to type to be an integer");

    if from_size < to_size {
        integer_extend(builder, &cast_from)
    } else if from_size > to_size {
        integer_truncate(builder, &cast_from.cast)
    } else {
        Ok(lower_expr(builder, &cast_from.cast.value)?)
    }
}
