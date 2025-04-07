use crate::parse::ParseError;
use c_token::Integer;
use primitives::{CInteger, IntegerRigidity, IntegerSign};
use source_files::Source;

pub fn translate_expr_integer(integer: &Integer, source: Source) -> Result<ast::Expr, ParseError> {
    let known = match integer {
        Integer::Int(x) => ast::IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Int, Some(IntegerSign::Signed)),
            value: (*x).into(),
        },
        Integer::UnsignedInt(x) => ast::IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Int, Some(IntegerSign::Unsigned)),
            value: (*x).into(),
        },
        Integer::Long(x) => ast::IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Long, Some(IntegerSign::Signed)),
            value: (*x).into(),
        },
        Integer::UnsignedLong(x) => ast::IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::Long, Some(IntegerSign::Unsigned)),
            value: (*x).into(),
        },
        Integer::LongLong(x) => ast::IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::LongLong, Some(IntegerSign::Signed)),
            value: (*x).into(),
        },
        Integer::UnsignedLongLong(x) => ast::IntegerKnown {
            rigidity: IntegerRigidity::Loose(CInteger::LongLong, Some(IntegerSign::Unsigned)),
            value: (*x).into(),
        },
    };

    Ok(ast::ExprKind::Integer(ast::Integer::Known(Box::new(known))).at(source))
}
