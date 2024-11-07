use crate::resolved::{self, Constraint};
use derive_more::IsVariant;
use indexmap::IndexMap;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct PolyType {
    constaints: HashSet<Constraint>,
}

#[derive(Clone, Debug)]
pub struct PolyExpr {
    expr: resolved::Expr,
}

#[derive(Clone, Debug, IsVariant)]
pub enum PolyValue {
    PolyType(PolyType),
    PolyExpr(PolyExpr),
}

#[derive(Clone, Debug)]
pub struct PolyCatalog {
    polymorphs: IndexMap<String, PolyValue>,
}

#[derive(Clone, Debug)]
pub enum PolyCatalogInsertError {
    /// Cannot have a single polymorph that is both a type as well as an expression
    Incongruent,
}

impl PolyCatalog {
    pub fn new() -> Self {
        Self {
            polymorphs: IndexMap::default(),
        }
    }

    pub fn put_type(
        &mut self,
        name: String,
        new_constraints: impl Iterator<Item = Constraint>,
    ) -> Result<(), PolyCatalogInsertError> {
        if let Some(existing) = self.polymorphs.get_mut(&name) {
            match existing {
                PolyValue::PolyType(poly_type) => {
                    poly_type.constaints.extend(new_constraints);
                }
                PolyValue::PolyExpr(_) => return Err(PolyCatalogInsertError::Incongruent),
            }
        } else {
            self.polymorphs.insert(
                name,
                PolyValue::PolyType(PolyType {
                    constaints: HashSet::from_iter(new_constraints),
                }),
            );
        }

        Ok(())
    }
}
