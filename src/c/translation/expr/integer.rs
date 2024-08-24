use crate::{
    ast::{self, CInteger, IntegerKnown, IntegerRigidity},
    c::{parser::ParseError, token::Integer},
    ir::IntegerSign,
    source_files::Source,
};

pub fn translate_expr_integer(integer: &Integer, source: Source) -> Result<ast::Expr, ParseError> {
    let known = match integer {
        Integer::Int(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Int),
            value: (*x).into(),
            sign: IntegerSign::Signed,
        },
        Integer::UnsignedInt(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Int),
            value: (*x).into(),
            sign: IntegerSign::Unsigned,
        },
        Integer::Long(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Long),
            value: (*x).into(),
            sign: IntegerSign::Signed,
        },
        Integer::UnsignedLong(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Long),
            value: (*x).into(),
            sign: IntegerSign::Unsigned,
        },
        Integer::LongLong(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::LongLong),
            value: (*x).into(),
            sign: IntegerSign::Signed,
        },
        Integer::UnsignedLongLong(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::LongLong),
            value: (*x).into(),
            sign: IntegerSign::Unsigned,
        },
    };

    Ok(ast::ExprKind::Integer(ast::Integer::Known(Box::new(known))).at(source))
}
