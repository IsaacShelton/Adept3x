use super::{PolyCatalogInsertError, PolyValue};
use crate::{
    asg::{Type, TypeKind},
    resolve::expr::ResolveExprCtx,
};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct TypePatternAttempt<'a> {
    pub pattern: &'a Type,
    pub concrete: &'a Type,
}

#[derive(Clone, Debug)]
pub enum MatchTypesError<'a> {
    LengthMismatch,
    NoMatch(TypePatternAttempt<'a>),
    Incongruent(TypePatternAttempt<'a>),
}

#[derive(Clone, Debug, Default)]
pub struct TypeMatch {
    pub addition: IndexMap<String, PolyValue>,
}

pub fn match_type<'t, 'a>(
    ctx: &'a ResolveExprCtx,
    polymorphs: &'a IndexMap<String, PolyValue>,
    pattern: &'t Type,
    concrete: &'t Type,
) -> Result<TypeMatch, MatchTypesError<'t>> {
    let mut matcher = TypeMatcher {
        ctx,
        parent: polymorphs,
        partial: Default::default(),
    };

    matcher.match_type(pattern, concrete)?;

    Ok(TypeMatch {
        addition: matcher.partial,
    })
}

pub struct TypeMatcher<'local, 'ast, 'root_ctx> {
    pub ctx: &'local ResolveExprCtx<'ast, 'root_ctx>,
    pub parent: &'local IndexMap<String, PolyValue>,
    pub partial: IndexMap<String, PolyValue>,
}

impl<'local, 'ast, 'root_ctx> TypeMatcher<'local, 'ast, 'root_ctx> {
    pub fn match_type<'t>(
        &mut self,
        pattern: &'t Type,
        concrete: &'t Type,
    ) -> Result<(), MatchTypesError<'t>> {
        let no_match = || {
            Err(MatchTypesError::NoMatch(TypePatternAttempt {
                concrete,
                pattern,
            }))
        };

        match &pattern.kind {
            TypeKind::Unresolved => panic!(),
            TypeKind::Boolean
            | TypeKind::Integer(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Floating(_)
            | TypeKind::Void
            | TypeKind::Never
            | TypeKind::Enum(_, _)
            | TypeKind::TypeAlias(_, _) => {
                if *pattern == *concrete {
                    Ok(())
                } else {
                    no_match()
                }
            }
            TypeKind::Trait(_, trait_ref, parameters) => match &concrete.kind {
                TypeKind::Trait(_, concrete_trait_ref, concrete_parameters) => {
                    if *trait_ref == *concrete_trait_ref
                        || parameters.len() != concrete_parameters.len()
                    {
                        return Err(MatchTypesError::Incongruent(TypePatternAttempt {
                            pattern,
                            concrete,
                        }));
                    }

                    for (pattern_parameter, concrete_parameter) in
                        parameters.iter().zip(concrete_parameters.iter())
                    {
                        self.match_type(pattern_parameter, concrete_parameter)?;
                    }

                    Ok(())
                }
                _ => no_match(),
            },
            TypeKind::Structure(_, struct_ref, parameters) => match &concrete.kind {
                TypeKind::Structure(_, concrete_struct_ref, concrete_parameters) => {
                    if *struct_ref != *concrete_struct_ref
                        || parameters.len() != concrete_parameters.len()
                    {
                        return no_match();
                    }

                    for (pattern_parameter, concrete_parameter) in
                        parameters.iter().zip(concrete_parameters.iter())
                    {
                        self.match_type(pattern_parameter, concrete_parameter)?;
                    }

                    Ok(())
                }
                _ => no_match(),
            },
            TypeKind::Ptr(pattern_inner) => match &concrete.kind {
                TypeKind::Ptr(concrete_inner) => self.match_type(pattern_inner, concrete_inner),
                _ => no_match(),
            },
            TypeKind::AnonymousStruct() => todo!(),
            TypeKind::AnonymousUnion() => todo!(),
            TypeKind::AnonymousEnum() => todo!(),
            TypeKind::FixedArray(pattern_inner) => match &concrete.kind {
                TypeKind::FixedArray(concrete_inner) => {
                    self.match_type(&pattern_inner.inner, &concrete_inner.inner)
                }
                _ => no_match(),
            },
            TypeKind::FuncPtr(_) => todo!(),
            TypeKind::Polymorph(name, constraints) => {
                self.put_type(name, concrete).map_err(|_| {
                    MatchTypesError::Incongruent(TypePatternAttempt { pattern, concrete })
                })?;

                for constraint in constraints {
                    if !self.ctx.current_constraints.satisfies(concrete, constraint) {
                        return no_match();
                    }
                }

                Ok(())
            }
        }
    }

    pub fn put_type(&mut self, name: &str, new_type: &Type) -> Result<(), PolyCatalogInsertError> {
        let in_parent = self.parent.get(name);
        if let Some(existing) = in_parent.or_else(|| self.partial.get(name)) {
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

        if in_parent.is_none() {
            assert!(self
                .partial
                .insert(name.to_string(), PolyValue::Type(new_type.clone()))
                .is_none());
        }

        Ok(())
    }
}
