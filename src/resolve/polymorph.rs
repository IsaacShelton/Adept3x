use super::expr::ResolveExprCtx;
use crate::{
    asg::{self, GenericTraitRef, Type},
    source_files::Source,
};
use core::hash::Hash;
use derive_more::IsVariant;
use indexmap::IndexMap;
use itertools::Itertools;
use std::fmt::Display;

// TODO: We probably want this to store some kind of internal hash
// Also, it should itself implement hash
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolyRecipe {
    pub polymorphs: IndexMap<String, PolyValue>,
}

impl From<IndexMap<String, Type>> for PolyRecipe {
    fn from(mut value: IndexMap<String, Type>) -> Self {
        Self {
            polymorphs: IndexMap::from_iter(
                value
                    .drain(..)
                    .map(|(name, ty)| (name, PolyValue::Type(ty))),
            ),
        }
    }
}

impl Display for PolyRecipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;

        for (i, (name, value)) in self.polymorphs.iter().enumerate() {
            write!(f, "${} :: ", name)?;

            match value {
                PolyValue::Type(ty) => {
                    write!(f, "{}", ty.to_string())?;
                }
                PolyValue::Expr(_) => {
                    todo!("mangle name for polymorphic function with expr polymorph")
                }
                PolyValue::Impl(impl_ref) => {
                    eprintln!(
                        "warning: name mangling for functions called with impl params is ad-hoc"
                    );
                    write!(f, "{:?}", impl_ref)?;
                }
            }

            if i + 1 != self.polymorphs.len() {
                write!(f, ", ")?;
            }
        }

        write!(f, ")")?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PolymorphError {
    pub kind: PolymorphErrorKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum PolymorphErrorKind {
    UndefinedPolymorph(String),
    PolymorphIsNotAType(String),
    PolymorphIsNotAnImpl(String),
}

impl PolymorphErrorKind {
    pub fn at(self, source: Source) -> PolymorphError {
        PolymorphError { kind: self, source }
    }
}

impl Display for PolymorphErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolymorphErrorKind::UndefinedPolymorph(name) => {
                write!(f, "Undefined polymorph '${}'", name)
            }
            PolymorphErrorKind::PolymorphIsNotAType(name) => {
                write!(f, "Polymorph '${}' is not a type", name)
            }
            PolymorphErrorKind::PolymorphIsNotAnImpl(name) => {
                write!(f, "Polymorph '${}' is not a trait implementation", name)
            }
        }
    }
}

impl PolyRecipe {
    pub fn resolve_type<'a>(&self, ty: &asg::Type) -> Result<asg::Type, PolymorphError> {
        let polymorphs = &self.polymorphs;

        Ok(match &ty.kind {
            asg::TypeKind::Unresolved => panic!(),
            asg::TypeKind::Boolean
            | asg::TypeKind::Integer(_, _)
            | asg::TypeKind::CInteger(_, _)
            | asg::TypeKind::IntegerLiteral(_)
            | asg::TypeKind::FloatLiteral(_)
            | asg::TypeKind::Void
            | asg::TypeKind::Never
            | asg::TypeKind::Floating(_) => ty.clone(),
            asg::TypeKind::Ptr(inner) => {
                asg::TypeKind::Ptr(Box::new(self.resolve_type(inner)?)).at(ty.source)
            }
            asg::TypeKind::AnonymousStruct() => todo!(),
            asg::TypeKind::AnonymousUnion() => todo!(),
            asg::TypeKind::AnonymousEnum() => todo!(),
            asg::TypeKind::FixedArray(fixed_array) => {
                asg::TypeKind::FixedArray(Box::new(asg::FixedArray {
                    size: fixed_array.size,
                    inner: self.resolve_type(&fixed_array.inner)?,
                }))
                .at(ty.source)
            }
            asg::TypeKind::FuncPtr(_) => todo!(),
            asg::TypeKind::Enum(_, _) => ty.clone(),
            asg::TypeKind::Structure(human_name, struct_ref, poly_args) => {
                let args = poly_args
                    .iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<Result<_, _>>()?;

                asg::TypeKind::Structure(human_name.clone(), *struct_ref, args).at(ty.source)
            }
            asg::TypeKind::TypeAlias(_, _) => ty.clone(),
            asg::TypeKind::Polymorph(name, _) => {
                let Some(value) = polymorphs.get(name) else {
                    return Err(PolymorphErrorKind::UndefinedPolymorph(name.clone()).at(ty.source));
                };

                let PolyValue::Type(poly_type) = value else {
                    return Err(PolymorphErrorKind::PolymorphIsNotAType(name.clone()).at(ty.source));
                };

                poly_type.clone()
            }
            asg::TypeKind::Trait(_, _, _) => ty.clone(),
        })
    }

    pub fn resolve_impl(&self, name: &str, source: Source) -> Result<asg::ImplRef, PolymorphError> {
        match self.polymorphs.get(name) {
            Some(PolyValue::Impl(impl_ref)) => Ok(*impl_ref),
            Some(_) => Err(PolymorphErrorKind::PolymorphIsNotAnImpl(name.into())),
            None => Err(PolymorphErrorKind::UndefinedPolymorph(name.into())),
        }
        .map_err(|err| err.at(source))
    }

    pub fn resolve_trait(
        &self,
        generic_trait: &GenericTraitRef,
    ) -> Result<GenericTraitRef, PolymorphError> {
        Ok(GenericTraitRef {
            trait_ref: generic_trait.trait_ref,
            args: generic_trait
                .args
                .iter()
                .map(|ty| self.resolve_type(ty))
                .try_collect()?,
        })
    }
}

impl Hash for PolyRecipe {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.polymorphs.len().hash(state);

        for (key, val) in self.polymorphs.iter() {
            key.hash(state);
            val.hash(state);
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, IsVariant)]
pub enum PolyValue {
    Type(asg::Type),
    Expr(asg::Expr),
    Impl(asg::ImplRef),
}

#[derive(Clone, Debug, Default)]
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

    pub fn bake(self) -> PolyRecipe {
        PolyRecipe {
            polymorphs: self.polymorphs,
        }
    }

    pub fn match_type(
        &mut self,
        ctx: &ResolveExprCtx,
        pattern_type: &Type,
        concrete_type: &Type,
    ) -> Result<(), Option<PolyCatalogInsertError>> {
        match &pattern_type.kind {
            asg::TypeKind::Unresolved => panic!(),
            asg::TypeKind::Boolean
            | asg::TypeKind::Integer(_, _)
            | asg::TypeKind::CInteger(_, _)
            | asg::TypeKind::IntegerLiteral(_)
            | asg::TypeKind::FloatLiteral(_)
            | asg::TypeKind::Floating(_)
            | asg::TypeKind::Void
            | asg::TypeKind::Never
            | asg::TypeKind::Enum(_, _)
            | asg::TypeKind::TypeAlias(_, _) => {
                if *pattern_type == *concrete_type {
                    Ok(())
                } else {
                    Err(None)
                }
            }
            asg::TypeKind::Trait(_, trait_ref, parameters) => match &concrete_type.kind {
                asg::TypeKind::Trait(_, concrete_trait_ref, concrete_parameters) => {
                    if *trait_ref == *concrete_trait_ref
                        || parameters.len() != concrete_parameters.len()
                    {
                        return Err(None);
                    }

                    for (pattern_parameter, concrete_parameter) in
                        parameters.iter().zip(concrete_parameters.iter())
                    {
                        self.match_type(ctx, pattern_parameter, concrete_parameter)?;
                    }

                    Ok(())
                }
                _ => Err(None),
            },
            asg::TypeKind::Structure(_, struct_ref, parameters) => match &concrete_type.kind {
                asg::TypeKind::Structure(_, concrete_struct_ref, concrete_parameters) => {
                    if *struct_ref != *concrete_struct_ref
                        || parameters.len() != concrete_parameters.len()
                    {
                        return Err(None);
                    }

                    for (pattern_parameter, concrete_parameter) in
                        parameters.iter().zip(concrete_parameters.iter())
                    {
                        self.match_type(ctx, pattern_parameter, concrete_parameter)?;
                    }

                    Ok(())
                }
                _ => Err(None),
            },
            asg::TypeKind::Ptr(pattern_inner) => match &concrete_type.kind {
                asg::TypeKind::Ptr(concrete_inner) => {
                    self.match_type(ctx, pattern_inner, concrete_inner)
                }
                _ => Err(None),
            },
            asg::TypeKind::AnonymousStruct() => todo!(),
            asg::TypeKind::AnonymousUnion() => todo!(),
            asg::TypeKind::AnonymousEnum() => todo!(),
            asg::TypeKind::FixedArray(pattern_inner) => match &concrete_type.kind {
                asg::TypeKind::FixedArray(concrete_inner) => {
                    self.match_type(ctx, &pattern_inner.inner, &concrete_inner.inner)
                }
                _ => Err(None),
            },
            asg::TypeKind::FuncPtr(_) => todo!(),
            asg::TypeKind::Polymorph(name, constraints) => {
                self.put_type(name, concrete_type).map_err(Some)?;

                for constraint in constraints {
                    if !ctx.current_constraints.satisfies(concrete_type, constraint) {
                        return Err(None);
                    }
                }

                Ok(())
            }
        }
    }

    pub fn put_type(&mut self, name: &str, new_type: &Type) -> Result<(), PolyCatalogInsertError> {
        if let Some(existing) = self.polymorphs.get_mut(name) {
            match existing {
                PolyValue::Type(poly_type) => {
                    if *poly_type != *new_type {
                        return Err(PolyCatalogInsertError::Incongruent);
                    }
                }
                PolyValue::Expr(_) => return Err(PolyCatalogInsertError::Incongruent),
                PolyValue::Impl(_) => return Err(PolyCatalogInsertError::Incongruent),
            }
        } else {
            self.polymorphs
                .insert(name.to_string(), PolyValue::Type(new_type.clone()));
        }

        Ok(())
    }

    pub fn get(&mut self, name: &str) -> Option<&PolyValue> {
        self.polymorphs.get(name)
    }
}
