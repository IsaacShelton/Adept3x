use super::{PolyRecipe, PolyValue};
use indexmap::IndexMap;

/// Mutable version of PolyRecipe
#[derive(Clone, Debug, Default)]
pub struct PolyCatalog<'env> {
    polymorphs: IndexMap<&'env str, PolyValue<'env>>,
}

impl<'env> PolyCatalog<'env> {
    pub fn overwrite(
        &mut self,
        name: &'env str,
        new_poly_value: PolyValue<'env>,
    ) -> Option<PolyValue<'env>> {
        self.polymorphs.insert(name, new_poly_value)
    }

    pub fn insert(&mut self, name: &'env str, new_poly_value: PolyValue<'env>) -> Result<(), ()> {
        self.can_insert(name, new_poly_value)?;
        self.polymorphs.insert(name, new_poly_value);
        Ok(())
    }

    pub fn can_insert(&self, name: &'env str, new_poly_value: PolyValue<'env>) -> Result<(), ()> {
        match self.polymorphs.get(name) {
            Some(PolyValue::Type(existing_type)) => match new_poly_value {
                PolyValue::Type(new_type) => {
                    if **existing_type != *new_type {
                        return Err(());
                    }
                }
            },
            None => (),
        }
        Ok(())
    }

    pub fn freeze(self) -> PolyRecipe<'env> {
        PolyRecipe::from_iter(self.polymorphs.into_iter())
    }
}
