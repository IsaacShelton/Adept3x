mod matcher;
mod type_args_to_poly_value;

use crate::expr::ResolveExprCtx;
use asg::{PolyCatalog, Type};
pub use matcher::MatchTypesError;
use matcher::{TypeMatcher, match_type};
pub use type_args_to_poly_value::*;

pub trait PolyCatalogExt {
    fn extend_if_match_type<'t>(
        &mut self,
        ctx: &ResolveExprCtx,
        pattern: &'t Type,
        concrete: &'t Type,
    ) -> Result<(), MatchTypesError<'t>>;

    fn extend_if_match_all_types<'slf: 'root_ctx, 't, 'expr_ctx, 'ast, 'root_ctx>(
        &'slf mut self,
        ctx: &'expr_ctx ResolveExprCtx<'ast, 'root_ctx>,
        pattern_types: &'t [Type],
        concrete_types: &'t [Type],
    ) -> Result<(), MatchTypesError<'t>>;

    fn try_match_all_types<'slf: 'root_ctx, 't, 'expr_ctx, 'ast, 'root_ctx>(
        &'slf self,
        ctx: &'expr_ctx ResolveExprCtx<'ast, 'root_ctx>,
        pattern_types: &'t [Type],
        concrete_types: &'t [Type],
    ) -> Result<TypeMatcher<'expr_ctx, 'ast, 'root_ctx>, MatchTypesError<'t>>;
}

impl PolyCatalogExt for PolyCatalog {
    fn extend_if_match_type<'t>(
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

    fn extend_if_match_all_types<'slf: 'root_ctx, 't, 'expr_ctx, 'ast, 'root_ctx>(
        &'slf mut self,
        ctx: &'expr_ctx ResolveExprCtx<'ast, 'root_ctx>,
        pattern_types: &'t [Type],
        concrete_types: &'t [Type],
    ) -> Result<(), MatchTypesError<'t>> {
        self.polymorphs.extend(
            self.try_match_all_types(ctx, pattern_types, concrete_types)?
                .partial,
        );
        Ok(())
    }

    fn try_match_all_types<'slf: 'root_ctx, 't, 'expr_ctx, 'ast, 'root_ctx>(
        &'slf self,
        ctx: &'expr_ctx ResolveExprCtx<'ast, 'root_ctx>,
        pattern_types: &'t [Type],
        concrete_types: &'t [Type],
    ) -> Result<TypeMatcher<'expr_ctx, 'ast, 'root_ctx>, MatchTypesError<'t>> {
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

        Ok(matcher)
    }
}
