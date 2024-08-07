use crate::{
    ast,
    c::{encoding::Encoding, parser::ParseError},
    source_files::Source,
};

pub fn translate_expr_string(
    encoding: &Encoding,
    content: &str,
    source: Source,
) -> Result<ast::Expr, ParseError> {
    if let Encoding::Default = encoding {
        return Ok(ast::ExprKind::String(content.into()).at(source));
    }

    todo!("translate non-default encoding C string")
}
