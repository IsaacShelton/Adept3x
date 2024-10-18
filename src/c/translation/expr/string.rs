use crate::{
    ast,
    c::{encoding::Encoding, parser::ParseError},
    source_files::Source,
};
use std::ffi::CString;

pub fn translate_expr_string(
    encoding: &Encoding,
    content: &str,
    source: Source,
) -> Result<ast::Expr, ParseError> {
    if let Encoding::Default = encoding {
        // TODO: Add proper error message?
        let content = CString::new(content).expect("valid null-terminated string");
        return Ok(ast::ExprKind::NullTerminatedString(content).at(source));
    }

    todo!("translate non-default encoding C string")
}
