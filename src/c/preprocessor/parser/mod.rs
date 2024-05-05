use super::{
    ast::{Group, PreprocessorAst},
    token::PreToken,
    PreprocessorError,
};

pub fn parse(_tokens: &Vec<Vec<PreToken>>) -> Result<PreprocessorAst, PreprocessorError> {
    let ast = PreprocessorAst {
        group: Group { groups: vec![] },
    };
    Ok(ast)
}
