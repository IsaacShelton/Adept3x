use crate::{
    ast::{self, IntegerKnown, IntegerSign, Source},
    c::{parser::ParseError, token::Integer},
    resolved::IntegerLiteralBits,
};

pub fn translate_expr_integer(integer: &Integer, source: Source) -> Result<ast::Expr, ParseError> {
    use IntegerLiteralBits::{Bits32, Bits64};
    use IntegerSign::{Signed, Unsigned};

    let ast_integer = match integer {
        Integer::Int(x) => ast::Integer::Known(Box::new(IntegerKnown {
            bits: Bits32,
            sign: Signed,
            value: (*x).into(),
        })),
        Integer::UnsignedInt(x) => ast::Integer::Known(Box::new(IntegerKnown {
            bits: Bits32,
            sign: Unsigned,
            value: (*x).into(),
        })),
        Integer::Long(x) | Integer::LongLong(x) => ast::Integer::Known(Box::new(IntegerKnown {
            bits: Bits64,
            sign: Signed,
            value: (*x).into(),
        })),
        Integer::UnsignedLong(x) | Integer::UnsignedLongLong(x) => {
            ast::Integer::Known(Box::new(IntegerKnown {
                bits: Bits64,
                sign: Unsigned,
                value: (*x).into(),
            }))
        }
    };

    Ok(ast::ExprKind::Integer(ast_integer).at(source))
}
