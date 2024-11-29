use super::ResolveTypeCtx;
use crate::{
    ast::{self, IntegerBits},
    ir::IntegerSign,
    resolve::error::{ResolveError, ResolveErrorKind},
    resolved::{self, Constraint},
    source_files::Source,
};
use std::borrow::Borrow;

impl<'a> ResolveTypeCtx<'a> {
    pub fn resolve_or_undeclared(
        &self,
        ast_type: &'a ast::Type,
    ) -> Result<resolved::Type, ResolveError> {
        match self.resolve(ast_type) {
            Ok(inner) => Ok(inner),
            Err(_) if ast_type.kind.allow_indirect_undefined() => {
                Ok(resolved::TypeKind::Void.at(ast_type.source))
            }
            Err(err) => Err(err),
        }
    }

    pub fn resolve(&self, ast_type: &'a ast::Type) -> Result<resolved::Type, ResolveError> {
        match &ast_type.kind {
            ast::TypeKind::Boolean => Ok(resolved::TypeKind::Boolean),
            ast::TypeKind::Integer(bits, sign) => Ok(resolved::TypeKind::Integer(*bits, *sign)),
            ast::TypeKind::CInteger(integer, sign) => {
                Ok(resolved::TypeKind::CInteger(*integer, *sign))
            }
            ast::TypeKind::Pointer(inner) => {
                let inner = self.resolve_or_undeclared(inner)?;
                Ok(resolved::TypeKind::Pointer(Box::new(inner)))
            }
            ast::TypeKind::Void => Ok(resolved::TypeKind::Void),
            ast::TypeKind::Named(name) => match self.find(name) {
                Ok(found) => Ok(found.into_owned()),
                Err(err) => Err(err.into_resolve_error(name, ast_type.source)),
            },
            ast::TypeKind::Floating(size) => Ok(resolved::TypeKind::Floating(*size)),
            ast::TypeKind::AnonymousStruct(..) => todo!("resolve anonymous struct type"),
            ast::TypeKind::AnonymousUnion(..) => todo!("resolve anonymous union type"),
            ast::TypeKind::AnonymousEnum(_) => {
                todo!("resolve anonymous enum type")
            }
            ast::TypeKind::FixedArray(fixed_array) => {
                if let ast::ExprKind::Integer(integer) = &fixed_array.count.kind {
                    if let Ok(size) = integer.value().try_into() {
                        let inner = self.resolve(&fixed_array.ast_type)?;

                        Ok(resolved::TypeKind::FixedArray(Box::new(
                            resolved::FixedArray { size, inner },
                        )))
                    } else {
                        Err(ResolveErrorKind::ArraySizeTooLarge.at(fixed_array.count.source))
                    }
                } else {
                    todo!("resolve fixed array type with variable size")
                }
            }
            ast::TypeKind::FunctionPointer(function_pointer) => {
                let mut parameters = Vec::with_capacity(function_pointer.parameters.len());

                for parameter in function_pointer.parameters.iter() {
                    let resolved_type = self.resolve(&parameter.ast_type)?;

                    parameters.push(resolved::Parameter {
                        name: parameter.name.clone(),
                        resolved_type,
                    });
                }

                let return_type = Box::new(self.resolve(&function_pointer.return_type)?);

                Ok(resolved::TypeKind::FunctionPointer(
                    resolved::FunctionPointer {
                        parameters,
                        return_type,
                        is_cstyle_variadic: function_pointer.is_cstyle_variadic,
                    },
                ))
            }
            ast::TypeKind::Polymorph(polymorph, constraints) => {
                let mut resolved_constraints = vec![];

                for constraint in constraints {
                    if let ast::TypeKind::Named(name) = &constraint.kind {
                        resolved_constraints.push(match name.as_plain_str() {
                            Some("PrimitiveAdd") => Constraint::PrimitiveAdd,
                            _ => {
                                return Err(ResolveErrorKind::UndeclaredTrait(name.to_string())
                                    .at(constraint.source))
                            }
                        });
                    }
                }

                Ok(resolved::TypeKind::Polymorph(
                    polymorph.clone(),
                    resolved_constraints,
                ))
            }
        }
        .map(|kind| kind.at(ast_type.source))
    }
}

fn resolve_enum_backing_type(
    ctx: &ResolveTypeCtx,
    backing_type: Option<impl Borrow<ast::Type>>,
    source: Source,
) -> Result<resolved::Type, ResolveError> {
    if let Some(backing_type) = backing_type.as_ref().map(Borrow::borrow) {
        ctx.resolve(backing_type)
    } else {
        Ok(resolved::TypeKind::Integer(IntegerBits::Bits64, IntegerSign::Unsigned).at(source))
    }
}
