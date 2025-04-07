use crate::{PolyRecipe, PolyValue, Type};
use indexmap::IndexMap;

#[derive(Clone, Debug, Default)]
pub struct PolyCatalog {
    pub polymorphs: IndexMap<String, PolyValue>,
}

impl PolyCatalog {
    pub fn new() -> Self {
        Self {
            polymorphs: IndexMap::default(),
        }
    }

    pub fn bake(self) -> PolyRecipe {
        PolyRecipe {
            polymorphs: self.polymorphs,
        }
    }

    pub fn can_put_type(
        &mut self,
        name: &str,
        new_type: &Type,
    ) -> Result<(), PolyCatalogInsertError> {
        if let Some(existing) = self.polymorphs.get_mut(name) {
            match existing {
                PolyValue::Type(poly_type) => {
                    if *poly_type != *new_type {
                        return Err(PolyCatalogInsertError::Incongruent);
                    }
                }
                PolyValue::Expr(_) | PolyValue::Impl(_) | PolyValue::PolyImpl(_) => {
                    return Err(PolyCatalogInsertError::Incongruent);
                }
            }
        }
        Ok(())
    }

    pub fn put_type(&mut self, name: &str, new_type: &Type) -> Result<(), PolyCatalogInsertError> {
        self.can_put_type(name, new_type)?;
        self.polymorphs
            .insert(name.to_string(), PolyValue::Type(new_type.clone()));
        Ok(())
    }

    pub fn get(&mut self, name: &str) -> Option<&PolyValue> {
        self.polymorphs.get(name)
    }
}

#[derive(Clone, Debug)]
pub enum PolyCatalogInsertError {
    /// Cannot have a single polymorph that is both a type as well as an expression
    Incongruent,
}
