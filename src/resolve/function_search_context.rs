use crate::{error::CompilerError, resolved};
use std::collections::HashMap;

use super::error::{ErrorInfo, ResolveError};

#[derive(Clone, Debug, Default)]
pub struct FunctionSearchContext {
    pub available: HashMap<String, Vec<resolved::FunctionRef>>,
}

impl FunctionSearchContext {
    pub fn find_function_or_error(
        &self,
        name: &str,
    ) -> Result<resolved::FunctionRef, ResolveError> {
        match self.find_function(name) {
            Some(function) => Ok(function),
            None => Err(ResolveError {
                filename: todo!(),
                location: todo!(),
                info: ErrorInfo::FailedToFindFunction {
                    name: name.to_string(),
                },
            }),
        }
    }

    pub fn find_function(&self, name: &str) -> Option<resolved::FunctionRef> {
        self.available
            .get(name)
            .and_then(|list| list.get(0))
            .copied()
    }
}
