use super::PolyCatalog;
use crate::{
    FixedArray, FuncPtr, GenericTraitRef, ImplRef, Param, Params, PolyRecipe, PolyValue,
    PolymorphError, PolymorphErrorKind, Type, TypeKind,
};
use indexmap::IndexMap;
use itertools::Itertools;
use source_files::Source;

pub trait IntoPolyRecipeResolver<'a> {
    fn resolver(&'a self) -> PolyRecipeResolver<'a>;
}

impl<'a> IntoPolyRecipeResolver<'a> for PolyRecipe {
    fn resolver(&'a self) -> PolyRecipeResolver<'a> {
        PolyRecipeResolver {
            direct: &self.polymorphs,
            parent: None,
        }
    }
}

impl<'a> IntoPolyRecipeResolver<'a> for PolyCatalog {
    fn resolver(&'a self) -> PolyRecipeResolver<'a> {
        PolyRecipeResolver {
            direct: &self.polymorphs,
            parent: None,
        }
    }
}

pub struct PolyRecipeResolver<'a> {
    direct: &'a IndexMap<String, PolyValue>,
    parent: Option<&'a PolyRecipeResolver<'a>>,
}

// NOTE: All operations assume disjointed-ness
impl<'a> PolyRecipeResolver<'a> {
    pub fn new(direct: &'a IndexMap<String, PolyValue>) -> Self {
        Self {
            direct,
            parent: None,
        }
    }

    pub fn new_disjoint(
        direct: &'a IndexMap<String, PolyValue>,
        parent: &'a PolyRecipeResolver<'a>,
    ) -> Self {
        for name in direct.keys() {
            if parent.get(name).is_some() {
                panic!(
                    "PolyRecipeResolver::new_disjoint called for non-disjoint values, '{}' exists in both",
                    name
                );
            }
        }

        Self {
            direct,
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<&PolyValue> {
        self.direct
            .get(name)
            .or_else(|| self.parent.and_then(|parent| parent.get(name)))
    }

    pub fn resolve_type(&self, ty: &Type) -> Result<Type, PolymorphError> {
        Ok(match &ty.kind {
            TypeKind::Unresolved => panic!("unresolved type"),
            TypeKind::Boolean
            | TypeKind::Integer(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::SizeInteger(_)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Void
            | TypeKind::Never
            | TypeKind::Floating(_) => ty.clone(),
            TypeKind::Ptr(inner) => {
                TypeKind::Ptr(Box::new(self.resolve_type(inner)?)).at(ty.source)
            }
            TypeKind::AnonymousStruct() => todo!(),
            TypeKind::AnonymousUnion() => todo!(),
            TypeKind::AnonymousEnum(_) => {
                // NOTE: There are no inner polymorphs
                ty.clone()
            }
            TypeKind::FixedArray(fixed_array) => TypeKind::FixedArray(Box::new(FixedArray {
                size: fixed_array.size,
                inner: self.resolve_type(&fixed_array.inner)?,
            }))
            .at(ty.source),
            TypeKind::FuncPtr(func) => {
                let required = func
                    .params
                    .required
                    .iter()
                    .map(|param| {
                        self.resolve_type(&param.ty).map(|ty| Param {
                            name: param.name.clone(),
                            ty,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let return_type = self.resolve_type(&func.return_type)?;

                TypeKind::FuncPtr(FuncPtr {
                    params: Params {
                        required,
                        is_cstyle_vararg: func.params.is_cstyle_vararg,
                    },
                    return_type: Box::new(return_type),
                })
                .at(ty.source)
            }
            TypeKind::Enum(_, _) => ty.clone(),
            TypeKind::Structure(human_name, struct_ref, poly_args) => {
                let args = poly_args
                    .iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<Result<_, _>>()?;

                TypeKind::Structure(human_name.clone(), *struct_ref, args).at(ty.source)
            }
            TypeKind::TypeAlias(human_name, type_alias_ref, poly_args) => {
                let args = poly_args
                    .iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<Result<_, _>>()?;

                TypeKind::TypeAlias(human_name.clone(), *type_alias_ref, args).at(ty.source)
            }
            TypeKind::Polymorph(name) => {
                let Some(value) = self.get(name) else {
                    return Err(PolymorphErrorKind::UndefinedPolymorph(name.clone()).at(ty.source));
                };

                let PolyValue::Type(poly_type) = value else {
                    return Err(PolymorphErrorKind::PolymorphIsNotAType(name.clone()).at(ty.source));
                };

                poly_type.clone()
            }
            TypeKind::Trait(_, _, _) => ty.clone(),
        })
    }

    pub fn resolve_impl(&self, name: &str, source: Source) -> Result<ImplRef, PolymorphError> {
        match self.get(name) {
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
