use crate::{
    ast::{self, CInteger, IntegerKnown, IntegerRigidity},
    c::{parser::ParseError, token::Integer},
    ir::IntegerSign,
    source_files::Source,
};

pub fn translate_expr_integer(integer: &Integer, source: Source) -> Result<ast::Expr, ParseError> {
    let known = match integer {
        Integer::Int(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Int, Some(IntegerSign::Signed)),
            value: (*x).into(),
        },
        Integer::UnsignedInt(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Int, Some(IntegerSign::Unsigned)),
            value: (*x).into(),
        },
        Integer::Long(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Long, Some(IntegerSign::Signed)),
            value: (*x).into(),
        },
        Integer::UnsignedLong(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Long, Some(IntegerSign::Unsigned)),
            value: (*x).into(),
        },
        Integer::LongLong(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::LongLong, Some(IntegerSign::Signed)),
            value: (*x).into(),
        },
        Integer::UnsignedLongLong(x) => IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::LongLong, Some(IntegerSign::Unsigned)),
            value: (*x).into(),
        },
    };

    Ok(ast::ExprKind::Integer(ast::Integer::Known(Box::new(known))).at(source))
}
