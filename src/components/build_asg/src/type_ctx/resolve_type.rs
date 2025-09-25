use super::ResolveTypeCtx;
use crate::{
    error::{ResolveError, ResolveErrorKind},
    unalias,
};
use asg::{AnonymousEnum, Params};
use primitives::{IntegerBits, IntegerSign};
use std::borrow::Borrow;

#[derive(Copy, Clone, Debug)]
pub enum ResolveTypeOptions {
    Unalias,
    KeepAliases,
}

impl<'a> ResolveTypeCtx<'a> {
    pub fn resolve_or_undeclared(
        &self,
        ast_type: &'a ast::Type,
        options: ResolveTypeOptions,
    ) -> Result<asg::Type, ResolveError> {
        match self.resolve(ast_type, options) {
            Ok(inner) => Ok(inner),
            Err(_) if ast_type.kind.allow_indirect_undefined() => {
                Ok(asg::TypeKind::Void.at(ast_type.source))
            }
            Err(err) => Err(err),
        }
    }

    pub fn resolve(
        &self,
        ast_type: &'a ast::Type,
        options: ResolveTypeOptions,
    ) -> Result<asg::Type, ResolveError> {
        let ty = self.resolve_keep_outer_aliases(ast_type, options)?;

        match options {
            ResolveTypeOptions::Unalias => Ok(unalias(self.asg, &ty)
                .map_err(ResolveErrorKind::from)
                .map_err(|e| e.at(ast_type.source))?
                .into_owned()),
            ResolveTypeOptions::KeepAliases => Ok(ty),
        }
    }

    pub fn resolve_keep_outer_aliases(
        &self,
        ast_type: &'a ast::Type,
        options: ResolveTypeOptions,
    ) -> Result<asg::Type, ResolveError> {
        match &ast_type.kind {
            ast::TypeKind::Boolean => Ok(asg::TypeKind::Boolean),
            ast::TypeKind::Integer(bits, sign) => Ok(asg::TypeKind::Integer(*bits, *sign)),
            ast::TypeKind::CInteger(integer, sign) => Ok(asg::TypeKind::CInteger(*integer, *sign)),
            ast::TypeKind::SizeInteger(sign) => Ok(asg::TypeKind::SizeInteger(*sign)),
            ast::TypeKind::Ptr(inner) => {
                let inner = self.resolve_or_undeclared(inner, options)?;
                Ok(asg::TypeKind::Ptr(Box::new(inner)))
            }
            ast::TypeKind::Deref(_) => {
                unimplemented!("deref'T is not supported for old compilation system")
            }
            ast::TypeKind::Void => Ok(asg::TypeKind::Void),
            ast::TypeKind::Never => Ok(asg::TypeKind::Never),
            ast::TypeKind::Named(name_path, arguments) => {
                match self.find(name_path, arguments, ast_type.source) {
                    Ok(found) => {
                        if let asg::TypeKind::Structure(_, struct_ref, arguments) = found.borrow() {
                            let structure = &self.asg.structs[*struct_ref];
                            assert!(arguments.len() == structure.params.len());
                        }

                        Ok(found.into_owned())
                    }
                    Err(err) => Err(err.into_resolve_error(name_path, ast_type.source)),
                }
            }
            ast::TypeKind::Floating(size) => Ok(asg::TypeKind::Floating(*size)),
            ast::TypeKind::AnonymousStruct(..) => todo!("resolve anonymous struct type"),
            ast::TypeKind::AnonymousUnion(..) => todo!("resolve anonymous union type"),
            ast::TypeKind::AnonymousEnum(enumeration) => {
                let backing_type = enumeration
                    .backing_type
                    .as_ref()
                    .map(|ty| self.resolve(ty.as_ref(), options))
                    .transpose()?
                    .unwrap_or_else(|| {
                        asg::TypeKind::Integer(IntegerBits::Bits32, IntegerSign::Signed)
                            .at(ast_type.source)
                    });

                Ok(asg::TypeKind::AnonymousEnum(Box::new(AnonymousEnum {
                    members: enumeration.members.clone(),
                    backing_type,
                    allow_implicit_integer_conversions: enumeration
                        .allow_implicit_integer_conversions,
                    source: ast_type.source,
                })))
            }
            ast::TypeKind::FixedArray(fixed_array) => {
                if let ast::ExprKind::Integer(integer) = &fixed_array.count.kind {
                    if let Ok(size) = integer.value().try_into() {
                        let inner = self.resolve(&fixed_array.ast_type, options)?;

                        Ok(asg::TypeKind::FixedArray(Box::new(asg::FixedArray {
                            size,
                            inner,
                        })))
                    } else {
                        Err(ResolveErrorKind::ArraySizeTooLarge.at(fixed_array.count.source))
                    }
                } else {
                    todo!("resolve fixed array type with variable size")
                }
            }
            ast::TypeKind::FuncPtr(function_pointer) => {
                let mut params = Vec::with_capacity(function_pointer.parameters.len());

                for param in function_pointer.parameters.iter() {
                    let ty = self.resolve(&param.ast_type, options)?;

                    params.push(asg::Param {
                        name: param.name.clone(),
                        ty,
                    });
                }

                let return_type = Box::new(self.resolve(&function_pointer.return_type, options)?);

                Ok(asg::TypeKind::FuncPtr(asg::FuncPtr {
                    return_type,
                    params: Params {
                        required: params,
                        is_cstyle_vararg: function_pointer.is_cstyle_variadic,
                    },
                }))
            }
            ast::TypeKind::Polymorph(polymorph) => Ok(asg::TypeKind::Polymorph(polymorph.clone())),
        }
        .map(|kind| kind.at(ast_type.source))
    }
}
