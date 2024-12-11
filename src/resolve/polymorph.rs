use super::expr::ResolveExprCtx;
use crate::{resolved, resolved::Type, source_files::Source};
use core::hash::Hash;
use derive_more::IsVariant;
use indexmap::IndexMap;
use std::fmt::Display;

// TODO: We probably want this to store some kind of internal hash
// Also, it should itself implement hash
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolyRecipe {
    pub polymorphs: IndexMap<String, PolyValue>,
}

impl Display for PolyRecipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;

        for (i, (name, value)) in self.polymorphs.iter().enumerate() {
            write!(f, "${} :: ", name)?;

            match value {
                PolyValue::PolyType(ty) => {
                    write!(f, "{}", ty.resolved_type.to_string())?;
                }
                PolyValue::PolyExpr(_) => {
                    todo!("mangle name for polymorphic function with expr polymorph")
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
        }
    }
}

impl PolyRecipe {
    pub fn resolve_type<'a>(&self, ty: &resolved::Type) -> Result<resolved::Type, PolymorphError> {
        let polymorphs = &self.polymorphs;

        Ok(match &ty.kind {
            resolved::TypeKind::Unresolved => panic!(),
            resolved::TypeKind::Boolean
            | resolved::TypeKind::Integer(_, _)
            | resolved::TypeKind::CInteger(_, _)
            | resolved::TypeKind::IntegerLiteral(_)
            | resolved::TypeKind::FloatLiteral(_)
            | resolved::TypeKind::Void
            | resolved::TypeKind::Floating(_) => ty.clone(),
            resolved::TypeKind::Pointer(inner) => {
                resolved::TypeKind::Pointer(Box::new(self.resolve_type(inner)?)).at(ty.source)
            }
            resolved::TypeKind::AnonymousStruct() => todo!(),
            resolved::TypeKind::AnonymousUnion() => todo!(),
            resolved::TypeKind::AnonymousEnum() => todo!(),
            resolved::TypeKind::FixedArray(fixed_array) => {
                resolved::TypeKind::FixedArray(Box::new(resolved::FixedArray {
                    size: fixed_array.size,
                    inner: self.resolve_type(&fixed_array.inner)?,
                }))
                .at(ty.source)
            }
            resolved::TypeKind::FunctionPointer(_) => todo!(),
            resolved::TypeKind::Enum(_, _) => ty.clone(),
            resolved::TypeKind::Structure(human_name, structure_ref, poly_args) => {
                let args = poly_args
                    .iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<Result<_, _>>()?;

                resolved::TypeKind::Structure(human_name.clone(), *structure_ref, args)
                    .at(ty.source)
            }
            resolved::TypeKind::TypeAlias(_, _) => ty.clone(),
            resolved::TypeKind::Polymorph(name, _) => {
                let Some(value) = polymorphs.get(name) else {
                    return Err(PolymorphErrorKind::UndefinedPolymorph(name.clone()).at(ty.source));
                };

                let PolyValue::PolyType(poly_type) = value else {
                    return Err(PolymorphErrorKind::PolymorphIsNotAType(name.clone()).at(ty.source));
                };

                poly_type.resolved_type.clone()
            }
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

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PolyType {
    pub resolved_type: Type,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PolyExpr {
    expr: resolved::Expr,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, IsVariant)]
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
            resolved::TypeKind::Unresolved => panic!(),
            resolved::TypeKind::Boolean
            | resolved::TypeKind::Integer(_, _)
            | resolved::TypeKind::CInteger(_, _)
            | resolved::TypeKind::IntegerLiteral(_)
            | resolved::TypeKind::FloatLiteral(_)
            | resolved::TypeKind::Floating(_)
            | resolved::TypeKind::Void
            | resolved::TypeKind::Enum(_, _)
            | resolved::TypeKind::TypeAlias(_, _) => {
                if *pattern_type == *concrete_type {
                    Ok(())
                } else {
                    Err(None)
                }
            }
            resolved::TypeKind::Structure(_, structure_ref, parameters) => {
                match &concrete_type.kind {
                    resolved::TypeKind::Structure(
                        _,
                        concrete_structure_ref,
                        concrete_parameters,
                    ) => {
                        if *structure_ref != *concrete_structure_ref
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
                }
            }
            resolved::TypeKind::Pointer(pattern_inner) => match &concrete_type.kind {
                resolved::TypeKind::Pointer(concrete_inner) => {
                    self.match_type(ctx, pattern_inner, concrete_inner)
                }
                _ => Err(None),
            },
            resolved::TypeKind::AnonymousStruct() => todo!(),
            resolved::TypeKind::AnonymousUnion() => todo!(),
            resolved::TypeKind::AnonymousEnum() => todo!(),
            resolved::TypeKind::FixedArray(pattern_inner) => match &concrete_type.kind {
                resolved::TypeKind::FixedArray(concrete_inner) => {
                    self.match_type(ctx, &pattern_inner.inner, &concrete_inner.inner)
                }
                _ => Err(None),
            },
            resolved::TypeKind::FunctionPointer(_) => todo!(),
            resolved::TypeKind::Polymorph(name, constraints) => {
                self.put_type(name, concrete_type).map_err(Some)?;

                for constraint in constraints {
                    if !ctx.constraints.satisfies(concrete_type, constraint) {
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
                PolyValue::PolyType(poly_type) => {
                    if poly_type.resolved_type != *new_type {
                        return Err(PolyCatalogInsertError::Incongruent);
                    }
                }
                PolyValue::PolyExpr(_) => return Err(PolyCatalogInsertError::Incongruent),
            }
        } else {
            self.polymorphs.insert(
                name.to_string(),
                PolyValue::PolyType(PolyType {
                    resolved_type: new_type.clone(),
                }),
            );
        }

        Ok(())
    }

    pub fn get(&mut self, name: &str) -> Option<&PolyValue> {
        self.polymorphs.get(name)
    }
}
