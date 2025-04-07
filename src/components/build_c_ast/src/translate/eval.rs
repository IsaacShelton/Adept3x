use crate::parse::{ParseError, error::ParseErrorKind};
use c_ast::{Expr, ExprKind};
use c_token::Encoding;
use num_bigint::BigInt;
use num_traits::Zero;

// NOTE: Should this combined with the delayed version that can happen during lowering?
pub fn evaluate_to_const_integer(expr: &Expr) -> Result<BigInt, ParseError> {
    match &expr.kind {
        ExprKind::Integer(integer) => {
            return Ok(integer.into());
        }
        ExprKind::Bool(x) => return Ok(BigInt::from(*x as i64)),
        ExprKind::Nullptr => return Ok(BigInt::zero()),
        ExprKind::Character(encoding, s) => match encoding {
            Encoding::Default => return Ok(BigInt::from(s.as_bytes()[0])),
            Encoding::Utf8 => (),
            Encoding::Utf16 => (),
            Encoding::Utf32 => (),
            Encoding::Wide => (),
        },
        ExprKind::BinaryOperation(_) => {
            todo!("binary operations not supported in constant integer expressions yet")
        }
        ExprKind::Ternary(_) => {
            todo!("ternary expressions not supported in constant integer expressions yet")
        }
        ExprKind::Cast(_) => todo!("type casts not supported in constant integer expressions yet"),
        ExprKind::Subscript(_) => {
            todo!("subscripts not supported in constant integer expressions yet")
        }
        ExprKind::Field(_) => {
            todo!("field accesses not supported in constant integer expressions yet")
        }
        ExprKind::Identifier(_) => {
            todo!("variables not supported in constant integer expressions yet")
        }
        ExprKind::EnumConstant(_, integer) => return Ok(integer.into()),
        ExprKind::Float(_, _)
        | ExprKind::StringLiteral(_, _)
        | ExprKind::Compound(_)
        | ExprKind::PreIncrement(_)
        | ExprKind::PreDecrement(_)
        | ExprKind::PostIncrement(_)
        | ExprKind::PostDecrement(_)
        | ExprKind::CompoundLiteral(_)
        | ExprKind::AddressOf(_)
        | ExprKind::Dereference(_)
        | ExprKind::Negate(_)
        | ExprKind::BitComplement(_)
        | ExprKind::Not(_)
        | ExprKind::Call(_, _)
        | ExprKind::SizeOf(_)
        | ExprKind::SizeOfValue(_)
        | ExprKind::AlignOf(_)
        | ExprKind::IntegerPromote(_) => (),
    }

    Err(ParseErrorKind::MustBeConstantInteger.at(expr.source))
}
