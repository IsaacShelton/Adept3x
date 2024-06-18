use crate::{
    ast::{self, IntegerSign, Source},
    c::{parser::ParseError, token::Integer},
    resolved::IntegerLiteralBits,
};

pub fn translate_expr_integer(integer: &Integer, source: Source) -> Result<ast::Expr, ParseError> {
    use IntegerLiteralBits::{Bits32, Bits64};
    use IntegerSign::{Signed, Unsigned};

    let ast_integer = match integer {
        Integer::Int(x) => ast::Integer::Known(Bits32, Signed, (*x).into()),
        Integer::UnsignedInt(x) => ast::Integer::Known(Bits32, Unsigned, (*x).into()),
        Integer::Long(x) | Integer::LongLong(x) => ast::Integer::Known(Bits64, Signed, (*x).into()),
        Integer::UnsignedLong(x) | Integer::UnsignedLongLong(x) => {
            ast::Integer::Known(Bits64, Unsigned, (*x).into())
        }
    };

    Ok(ast::ExprKind::Integer(ast_integer).at(source))
}
