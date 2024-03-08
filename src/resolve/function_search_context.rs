use crate::{error::CompilerError, resolved};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct FunctionSearchContext {
    pub available: HashMap<String, Vec<resolved::FunctionRef>>,
}

impl FunctionSearchContext {
    pub fn find_function_or_error(
        &self,
        name: &str,
    ) -> Result<resolved::FunctionRef, CompilerError> {
        match self.find_function(name) {
            Some(function) => Ok(function),
            None => Err(CompilerError::during_resolve(format!(
                "Failed to find function '{}'",
                name
            ))),
        }
    }

    pub fn find_function(&self, name: &str) -> Option<resolved::FunctionRef> {
        self.available
            .get(name)
            .and_then(|list| list.get(0))
            .copied()
    }
}
