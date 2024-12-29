use super::{resolve_constraints, ResolveTypeCtx};
use crate::{
    asg::{self},
    ast::{self, IntegerBits},
    ir::IntegerSign,
    resolve::error::{ResolveError, ResolveErrorKind},
    source_files::Source,
};
use std::borrow::Borrow;

impl<'a> ResolveTypeCtx<'a> {
    pub fn resolve_or_undeclared(
        &self,
        ast_type: &'a ast::Type,
    ) -> Result<asg::Type, ResolveError> {
        match self.resolve(ast_type) {
            Ok(inner) => Ok(inner),
            Err(_) if ast_type.kind.allow_indirect_undefined() => {
                Ok(asg::TypeKind::Void.at(ast_type.source))
            }
            Err(err) => Err(err),
        }
    }

    pub fn resolve(&self, ast_type: &'a ast::Type) -> Result<asg::Type, ResolveError> {
        match &ast_type.kind {
            ast::TypeKind::Boolean => Ok(asg::TypeKind::Boolean),
            ast::TypeKind::Integer(bits, sign) => Ok(asg::TypeKind::Integer(*bits, *sign)),
            ast::TypeKind::CInteger(integer, sign) => Ok(asg::TypeKind::CInteger(*integer, *sign)),
            ast::TypeKind::Pointer(inner) => {
                let inner = self.resolve_or_undeclared(inner)?;
                Ok(asg::TypeKind::Pointer(Box::new(inner)))
            }
            ast::TypeKind::Void => Ok(asg::TypeKind::Void),
            ast::TypeKind::Named(name, arguments) => match self.find(name, arguments) {
                Ok(found) => {
                    if let asg::TypeKind::Structure(_, struct_ref, arguments) = found.borrow() {
                        let structure = self
                            .asg
                            .structs
                            .get(*struct_ref)
                            .expect("referenced struct to exist");

                        assert!(arguments.len() == structure.params.len());

                        for (parameter, argument) in
                            structure.params.parameters.values().zip(arguments)
                        {
                            for constraint in &parameter.constraints {
                                if !self.current_constraints.satisfies(argument, constraint) {
                                    return Err(ResolveErrorKind::ConstraintsNotSatisfiedForType {
                                        name: name.to_string(),
                                    }
                                    .at(ast_type.source));
                                }
                            }
                        }
                    }

                    Ok(found.into_owned())
                }
                Err(err) => Err(err.into_resolve_error(name, ast_type.source)),
            },
            ast::TypeKind::Floating(size) => Ok(asg::TypeKind::Floating(*size)),
            ast::TypeKind::AnonymousStruct(..) => todo!("resolve anonymous struct type"),
            ast::TypeKind::AnonymousUnion(..) => todo!("resolve anonymous union type"),
            ast::TypeKind::AnonymousEnum(_) => {
                todo!("resolve anonymous enum type")
            }
            ast::TypeKind::FixedArray(fixed_array) => {
                if let ast::ExprKind::Integer(integer) = &fixed_array.count.kind {
                    if let Ok(size) = integer.value().try_into() {
                        let inner = self.resolve(&fixed_array.ast_type)?;

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
            ast::TypeKind::FunctionPointer(function_pointer) => {
                let mut parameters = Vec::with_capacity(function_pointer.parameters.len());

                for parameter in function_pointer.parameters.iter() {
                    let ty = self.resolve(&parameter.ast_type)?;

                    parameters.push(asg::Parameter {
                        name: parameter.name.clone(),
                        ty,
                    });
                }

                let return_type = Box::new(self.resolve(&function_pointer.return_type)?);

                Ok(asg::TypeKind::FuncPointer(asg::FuncPtr {
                    parameters,
                    return_type,
                    is_cstyle_variadic: function_pointer.is_cstyle_variadic,
                }))
            }
            ast::TypeKind::Polymorph(polymorph, constraints) => Ok(asg::TypeKind::Polymorph(
                polymorph.clone(),
                resolve_constraints(self, constraints)?,
            )),
        }
        .map(|kind| kind.at(ast_type.source))
    }
}

fn resolve_enum_backing_type(
    ctx: &ResolveTypeCtx,
    backing_type: Option<impl Borrow<ast::Type>>,
    source: Source,
) -> Result<asg::Type, ResolveError> {
    if let Some(backing_type) = backing_type.as_ref().map(Borrow::borrow) {
        ctx.resolve(backing_type)
    } else {
        Ok(asg::TypeKind::Integer(IntegerBits::Bits64, IntegerSign::Unsigned).at(source))
    }
}
