use crate::{
    name::{Name, ResolvedName},
    resolved,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FunctionSearchCtx {
    pub available: HashMap<ResolvedName, Vec<resolved::FunctionRef>>,
    pub imported_namespaces: Vec<Box<str>>,
}

#[derive(Clone, Debug)]
pub enum FindFunctionError {
    NotDefined,
    Ambiguous,
}

impl FunctionSearchCtx {
    pub fn new(imported_namespaces: Vec<Box<str>>) -> Self {
        Self {
            available: Default::default(),
            imported_namespaces,
        }
    }

    pub fn find_function(&self, name: &Name) -> Result<resolved::FunctionRef, FindFunctionError> {
        eprintln!("warning: function call name resolution not fully implemented yet");

        let resolved_name = if !name.namespace.is_empty() {
            ResolvedName::Project(format!("{}{}", name.namespace, name.basename).into_boxed_str())
        } else {
            ResolvedName::Project(name.basename.clone().into_boxed_str())
        };

        if let Some(found) = self
            .available
            .get(&resolved_name)
            .and_then(|list| list.first())
            .copied()
        {
            return Ok(found);
        }

        if name.namespace.is_empty() {
            let mut matches = self.imported_namespaces.iter().filter_map(|namespace| {
                self.available
                    .get(&ResolvedName::Project(
                        format!("{}/{}", namespace, name.basename).into_boxed_str(),
                    ))
                    .and_then(|list| list.first())
                    .copied()
            });

            if let Some(found) = matches.next() {
                if matches.next().is_some() {
                    return Err(FindFunctionError::Ambiguous);
                } else {
                    return Ok(found);
                }
            }
        }

        Err(FindFunctionError::NotDefined)
    }
}
