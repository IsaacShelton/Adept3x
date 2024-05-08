use super::{depleted::Depleted, region::expand_region, Environment, Token};
use crate::c::preprocessor::{
    pre_token::{PreToken, PreTokenKind},
    PreprocessorError,
};

pub fn expand_include(
    files: &[PreToken],
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    let files = expand_region(files, environment, depleted)?;

    if files.len() != 1 {
        return Err(PreprocessorError::BadInclude);
    }

    // We can choose to satisfy these includes however we want
    match &files.first().unwrap().kind {
        PreTokenKind::HeaderName(header_name) => eprintln!("including <{}>", header_name),
        PreTokenKind::StringLiteral(_encoding, header_name) => {
            eprintln!("including \"{}\"", header_name)
        }
        _ => return Err(PreprocessorError::BadInclude),
    }

    Ok(vec![])
}
