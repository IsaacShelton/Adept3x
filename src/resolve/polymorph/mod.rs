mod error;
mod matcher;
mod recipe;
mod resolver;

use super::expr::ResolveExprCtx;
use crate::asg::{self, Type};
use core::hash::Hash;
use derive_more::IsVariant;
pub use error::*;
use indexmap::IndexMap;
pub use matcher::MatchTypesError;
use matcher::{match_type, TypeMatcher};
pub use recipe::PolyRecipe;
pub use resolver::PolyRecipeResolver;

#[derive(Clone, Debug, Hash, PartialEq, Eq, IsVariant)]
pub enum PolyValue {
    Type(asg::Type),
    Expr(asg::Expr),
    Impl(asg::ImplRef),
    PolyImpl(String),
}

#[derive(Clone, Debug, Default)]
pub struct PolyCatalog {
    pub polymorphs: IndexMap<String, PolyValue>,
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

    pub fn resolver<'a>(&'a self) -> PolyRecipeResolver<'a> {
        PolyRecipeResolver::new(&self.polymorphs)
    }

    pub fn bake(self) -> PolyRecipe {
        PolyRecipe {
            polymorphs: self.polymorphs,
        }
    }

    pub fn extend_if_match_type<'t>(
        &mut self,
        ctx: &ResolveExprCtx,
        pattern: &'t Type,
        concrete: &'t Type,
    ) -> Result<(), MatchTypesError<'t>> {
        self.polymorphs.extend(
            match_type(ctx, &self.polymorphs, pattern, concrete)?
                .addition
                .into_iter(),
        );
        Ok(())
    }

    pub fn extend_if_match_all_types<'t>(
        &mut self,
        ctx: &ResolveExprCtx,
        pattern_types: &'t [Type],
        concrete_types: &'t [Type],
    ) -> Result<(), MatchTypesError<'t>> {
        if concrete_types.len() != pattern_types.len() {
            return Err(MatchTypesError::LengthMismatch);
        }

        let mut matcher = TypeMatcher {
            ctx,
            parent: &self.polymorphs,
            partial: Default::default(),
        };

        for (pattern, concrete) in pattern_types.iter().zip(concrete_types.iter()) {
            matcher.match_type(pattern, concrete)?;
        }

        self.polymorphs.extend(matcher.partial.into_iter());
        Ok(())
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
                    return Err(PolyCatalogInsertError::Incongruent)
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
