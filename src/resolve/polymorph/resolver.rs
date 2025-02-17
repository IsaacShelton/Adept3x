use super::{error::PolymorphErrorKind, PolyValue, PolymorphError};
use crate::{
    asg::{self, GenericTraitRef},
    source_files::Source,
};
use indexmap::IndexMap;
use itertools::Itertools;

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
                panic!("PolyRecipeResolver::new_disjoint called for non-disjoint values, '{}' exists in both", name);
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

    pub fn resolve_type(&self, ty: &asg::Type) -> Result<asg::Type, PolymorphError> {
        Ok(match &ty.kind {
            asg::TypeKind::Unresolved => panic!("unresolved type"),
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
            asg::TypeKind::AnonymousEnum(_) => {
                // NOTE: There are no inner polymorphs
                ty.clone()
            }
            asg::TypeKind::FixedArray(fixed_array) => {
                asg::TypeKind::FixedArray(Box::new(asg::FixedArray {
                    size: fixed_array.size,
                    inner: self.resolve_type(&fixed_array.inner)?,
                }))
                .at(ty.source)
            }
            asg::TypeKind::FuncPtr(func) => {
                let required = func
                    .params
                    .required
                    .iter()
                    .map(|param| {
                        self.resolve_type(&param.ty).map(|ty| asg::Param {
                            name: param.name.clone(),
                            ty,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let return_type = self.resolve_type(&func.return_type)?;

                asg::TypeKind::FuncPtr(asg::FuncPtr {
                    params: asg::Params {
                        required,
                        is_cstyle_vararg: func.params.is_cstyle_vararg,
                    },
                    return_type: Box::new(return_type),
                })
                .at(ty.source)
            }
            asg::TypeKind::Enum(_, _) => ty.clone(),
            asg::TypeKind::Structure(human_name, struct_ref, poly_args) => {
                let args = poly_args
                    .iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<Result<_, _>>()?;

                asg::TypeKind::Structure(human_name.clone(), *struct_ref, args).at(ty.source)
            }
            asg::TypeKind::TypeAlias(human_name, type_alias_ref, poly_args) => {
                let args = poly_args
                    .iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<Result<_, _>>()?;

                asg::TypeKind::TypeAlias(human_name.clone(), *type_alias_ref, args).at(ty.source)
            }
            asg::TypeKind::Polymorph(name) => {
                let Some(value) = self.get(name) else {
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
