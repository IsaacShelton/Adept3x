use crate::{
    ast,
    c::{parser::ParseError, token::Integer},
    source_files::Source,
};

pub fn translate_expr_integer(integer: &Integer, source: Source) -> Result<ast::Expr, ParseError> {
    use ast::{
        IntegerFixedBits::{Bits32, Bits64},
        IntegerKnown,
        IntegerSign::{Signed, Unsigned},
    };

    let known = match integer {
        Integer::Int(x) => IntegerKnown {
            bits: Bits32,
            sign: Signed,
            value: (*x).into(),
        },
        Integer::UnsignedInt(x) => IntegerKnown {
            bits: Bits32,
            sign: Unsigned,
            value: (*x).into(),
        },
        Integer::Long(x) | Integer::LongLong(x) => IntegerKnown {
            bits: Bits64,
            sign: Signed,
            value: (*x).into(),
        },
        Integer::UnsignedLong(x) | Integer::UnsignedLongLong(x) => IntegerKnown {
            bits: Bits64,
            sign: Unsigned,
            value: (*x).into(),
        },
    };

    Ok(ast::ExprKind::Integer(ast::Integer::Known(Box::new(known))).at(source))
}
