use super::{depleted::Depleted, region::expand_region, Environment};
use crate::{
    c::preprocessor::{
        error::PreprocessorErrorKind,
        pre_token::{PreToken, PreTokenKind},
        PreprocessorError,
    },
    source_files::Source,
};

pub fn expand_include(
    files: &[PreToken],
    environment: &mut Environment,
    depleted: &mut Depleted,
    source: Source,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let files = expand_region(files, environment, depleted)?;

    if files.len() != 1 {
        return Err(PreprocessorErrorKind::BadInclude.at(source));
    }

    // We can choose to satisfy these includes however we want
    match &files.first().unwrap().kind {
        PreTokenKind::HeaderName(header_name) => eprintln!("including <{}>", header_name),
        PreTokenKind::StringLiteral(_encoding, header_name) => {
            eprintln!("including \"{}\"", header_name)
        }
        _ => return Err(PreprocessorErrorKind::BadInclude.at(source)),
    }

    Ok(vec![])
}
