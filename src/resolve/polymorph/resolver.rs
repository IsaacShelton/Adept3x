use super::{error::PolymorphErrorKind, PolyValue, PolymorphError};
use crate::{
    asg::{self, GenericTraitRef},
    source_files::Source,
};
use indexmap::IndexMap;
use itertools::Itertools;

pub struct PolyRecipeResolver<'a> {
    polymorphs: &'a IndexMap<String, PolyValue>,
}

impl<'a> PolyRecipeResolver<'a> {
    pub fn new(polymorphs: &'a IndexMap<String, PolyValue>) -> Self {
        Self { polymorphs }
    }
    pub fn resolve_type(&self, ty: &asg::Type) -> Result<asg::Type, PolymorphError> {
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
