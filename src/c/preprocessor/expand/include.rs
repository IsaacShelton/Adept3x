use std::num::NonZeroU32;

use super::{depleted::Depleted, region::expand_region, Environment};
use crate::c::preprocessor::{
    pre_token::{PreToken, PreTokenKind},
    PreprocessorError, PreprocessorErrorKind,
};

pub fn expand_include(
    files: &[PreToken],
    environment: &mut Environment,
    depleted: &mut Depleted,
    line: Option<NonZeroU32>,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let files = expand_region(files, environment, depleted)?;

    if files.len() != 1 {
        return Err(PreprocessorErrorKind::BadInclude.at(line));
    }

    // We can choose to satisfy these includes however we want
    match &files.first().unwrap().kind {
        PreTokenKind::HeaderName(header_name) => eprintln!("including <{}>", header_name),
        PreTokenKind::StringLiteral(_encoding, header_name) => {
            eprintln!("including \"{}\"", header_name)
        }
        _ => return Err(PreprocessorErrorKind::BadInclude.at(line)),
    }

    Ok(vec![])
}
