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
            | TypeKind::SizeInteger(_)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Floating(_)
            | TypeKind::Void
            | TypeKind::Never
            | TypeKind::Enum(_, _) => {
                if pattern.kind == concrete.kind {
                    Ok(())
                } else {
                    no_match()
                }
            }
            TypeKind::TypeAlias(_, type_alias_ref, params) => match &concrete.kind {
                TypeKind::TypeAlias(_, concrete_type_alias_ref, concrete_params) => {
                    if *type_alias_ref == *concrete_type_alias_ref
                        || params.len() != concrete_params.len()
                    {
                        return Err(MatchTypesError::Incongruent(TypePatternAttempt {
                            pattern,
                            concrete,
                        }));
                    }

                    for (pattern_param, concrete_param) in params.iter().zip(concrete_params.iter())
                    {
                        self.match_type(pattern_param, concrete_param)?;
                    }

                    Ok(())
                }
                _ => no_match(),
            },
            TypeKind::Trait(_, trait_ref, params) => match &concrete.kind {
                TypeKind::Trait(_, concrete_trait_ref, concrete_params) => {
                    if *trait_ref == *concrete_trait_ref || params.len() != concrete_params.len() {
                        return Err(MatchTypesError::Incongruent(TypePatternAttempt {
                            pattern,
                            concrete,
                        }));
                    }

                    for (pattern_param, concrete_param) in params.iter().zip(concrete_params.iter())
                    {
                        self.match_type(pattern_param, concrete_param)?;
                    }

                    Ok(())
                }
                _ => no_match(),
            },
            TypeKind::Structure(_, struct_ref, params) => match &concrete.kind {
                TypeKind::Structure(_, concrete_struct_ref, concrete_params) => {
                    if *struct_ref != *concrete_struct_ref || params.len() != concrete_params.len()
                    {
                        return no_match();
                    }

                    for (pattern_param, concrete_param) in params.iter().zip(concrete_params.iter())
                    {
                        self.match_type(pattern_param, concrete_param)?;
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
            TypeKind::AnonymousEnum(pattern_inner) => match &concrete.kind {
                TypeKind::AnonymousEnum(concrete_inner) => {
                    // NOTE: Can never be polymorphic, so ok
                    if pattern_inner.backing_type != concrete_inner.backing_type
                        || pattern_inner.members.len() != concrete_inner.members.len()
                    {
                        return no_match();
                    }

                    for ((pattern_name, pattern_value), (concrete_name, concrete_value)) in
                        pattern_inner
                            .members
                            .iter()
                            .zip(concrete_inner.members.iter())
                    {
                        // NOTE: We are only checking the actual values match,
                        // should we care if the explicitness between them is different too?
                        if pattern_name != concrete_name
                            || pattern_value.value == concrete_value.value
                        {
                            return no_match();
                        }
                    }

                    Ok(())
                }
                _ => no_match(),
            },
            TypeKind::FixedArray(pattern_inner) => match &concrete.kind {
                TypeKind::FixedArray(concrete_inner) => {
                    self.match_type(&pattern_inner.inner, &concrete_inner.inner)
                }
                _ => no_match(),
            },
            TypeKind::FuncPtr(_) => todo!(),
            TypeKind::Polymorph(name) => self.put_type(name, concrete).map_err(|_| {
                MatchTypesError::Incongruent(TypePatternAttempt { pattern, concrete })
            }),
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
