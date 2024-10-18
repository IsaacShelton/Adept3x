use crate::{
    ast,
    c::{
        encoding::Encoding,
        parser::{error::ParseErrorKind, ParseError},
    },
    source_files::Source,
};
use std::ffi::CString;

pub fn translate_expr_string(
    encoding: &Encoding,
    content: &str,
    source: Source,
) -> Result<ast::Expr, ParseError> {
    if let Encoding::Default = encoding {
        let Ok(content) = CString::new(content) else {
            return Err(ParseErrorKind::CannotContainNulInNullTerminatedString.at(source));
        };
        return Ok(ast::ExprKind::NullTerminatedString(content).at(source));
    }

    todo!("translate non-default encoding C string")
}
