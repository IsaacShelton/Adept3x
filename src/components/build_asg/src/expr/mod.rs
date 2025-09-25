mod array_access;
mod basic_binary_operation;
mod call;
mod conditional;
mod declare_assign;
mod member_expr;
mod short_circuiting_binary_operation;
mod static_member;
mod struct_literal;
mod unary_operation;
mod variable;

use self::{
    array_access::resolve_array_access_expr,
    basic_binary_operation::resolve_basic_binary_operation_expr,
    short_circuiting_binary_operation::resolve_short_circuiting_binary_operation_expr,
};
use super::{
    Initialized, ResolveTypeCtx,
    conform::{ConformMode, conform_expr_or_error},
    destination::resolve_expr_to_destination,
    error::ResolveError,
    func_haystack::FuncHaystack,
    type_ctx::ResolveTypeOptions,
    variable_haystack::VariableHaystack,
};
use crate::{error::ResolveErrorKind, resolve_stmts};
use asg::{Asg, Cast, CastFrom, Expr, ExprKind, FuncRef, StructRef, TypeKind, TypedExpr};
use ast::{ConformBehavior, IntegerKnown, Language, UnaryOperator};
use ast_workspace_settings::Settings;
pub use basic_binary_operation::resolve_basic_binary_operator;
use call::resolve_call_expr;
use conditional::resolve_conditional_expr;
use data_units::BitUnits;
use declare_assign::resolve_declare_assign_expr;
use fs_tree::FsNodeId;
use member_expr::resolve_member_expr;
use ordered_float::NotNan;
use primitives::{
    CInteger, CIntegerAssumptions, FloatSize, IntegerBits, IntegerRigidity, IntegerSign,
};
use static_member::{resolve_static_member_call, resolve_static_member_value};
use std::collections::HashMap;
use struct_literal::resolve_struct_literal_expr;
use unary_operation::resolve_unary_math_operation_expr;
use variable::resolve_variable_expr;

pub struct ResolveExprCtx<'ast, 'root_ctx> {
    pub asg: &'root_ctx mut Asg<'ast>,
    pub func_haystack: &'root_ctx FuncHaystack,
    pub variable_haystack: VariableHaystack,
    pub func_ref: Option<asg::FuncRef>,
    pub settings: &'root_ctx Settings,
    pub public_funcs: &'root_ctx HashMap<FsNodeId, HashMap<String, Vec<asg::FuncRef>>>,
    pub types_in_modules: &'root_ctx HashMap<FsNodeId, HashMap<String, asg::TypeDecl>>,
    pub globals_in_modules: &'root_ctx HashMap<FsNodeId, HashMap<String, asg::GlobalDecl>>,
    pub helper_exprs_in_modules: &'root_ctx HashMap<FsNodeId, HashMap<String, asg::HelperExprDecl>>,
    pub impls_in_modules: &'root_ctx HashMap<FsNodeId, HashMap<String, asg::ImplDecl>>,
    pub module_fs_node_id: FsNodeId,
    pub physical_fs_node_id: FsNodeId,
}

impl<'a, 'b> ResolveExprCtx<'a, 'b> {
    pub fn type_ctx<'c>(&'c self) -> ResolveTypeCtx<'c> {
        ResolveTypeCtx::from(self)
    }

    pub fn adept_conform_behavior(&self) -> ConformBehavior {
        ConformBehavior::Adept(self.c_integer_assumptions())
    }

    pub fn conform_behavior(&self, language: Language) -> ConformBehavior {
        match language {
            Language::Adept => self.adept_conform_behavior(),
            Language::C => ConformBehavior::C,
        }
    }

    pub fn c_integer_assumptions(&self) -> CIntegerAssumptions {
        self.settings.c_integer_assumptions()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ResolveExprMode {
    RequireValue,
    NeglectValue,
}

#[derive(Copy, Clone, Debug)]
pub enum PreferredType<'a> {
    Reference(&'a asg::Type),
    ParameterType(FuncRef, usize),
    ReturnType(FuncRef),
    FieldType(StructRef, &'a str),
}

impl<'a> PreferredType<'a> {
    pub fn of(reference: &'a asg::Type) -> Self {
        Self::Reference(reference)
    }

    pub fn of_parameter(func_ref: FuncRef, index: usize) -> Self {
        Self::ParameterType(func_ref, index)
    }

    pub fn view(&self, asg: &'a Asg) -> &'a asg::Type {
        match self {
            PreferredType::Reference(reference) => reference,
            PreferredType::ParameterType(func_ref, index) => {
                &asg.funcs[*func_ref].params.required.get(*index).unwrap().ty
            }
            PreferredType::ReturnType(func_ref) => &asg.funcs[*func_ref].return_type,
            PreferredType::FieldType(struct_ref, field_name) => {
                let (_, _, field) = asg.structs[*struct_ref]
                    .fields
                    .get_full::<str>(field_name)
                    .expect("referenced struct field type to exist");

                &field.ty
            }
        }
    }
}

pub fn resolve_expr(
    ctx: &mut ResolveExprCtx,
    ast_expr: &ast::Expr,
    preferred_type: Option<PreferredType>,
    initialized: Initialized,
    mode: ResolveExprMode,
) -> Result<asg::TypedExpr, ResolveError> {
    let source = ast_expr.source;

    let resolved_expr = match &ast_expr.kind {
        ast::ExprKind::Variable(name_path) => {
            resolve_variable_expr(ctx, name_path, preferred_type, initialized, mode, source)
        }
        ast::ExprKind::Char(content) => {
            if content.len() == 1 {
                if let Some(preferred_type) = preferred_type {
                    if let TypeKind::CInteger(CInteger::Char, _) = preferred_type.view(ctx.asg).kind
                    {
                        let expr = asg::ExprKind::IntegerKnown(Box::new(IntegerKnown {
                            rigidity: IntegerRigidity::Loose(CInteger::Char, None),
                            value: content.as_bytes()[0].into(),
                        }))
                        .at(source);

                        return Ok(TypedExpr::new(
                            asg::TypeKind::CInteger(CInteger::Char, None).at(source),
                            expr,
                        ));
                    }
                }
            }

            Err(ResolveErrorKind::UndeterminedCharacterLiteral.at(source))
        }
        ast::ExprKind::Integer(value) => {
            let (ty, expr) = match value {
                ast::Integer::Known(known) => (
                    asg::TypeKind::from(known.as_ref()).at(source),
                    asg::ExprKind::IntegerKnown(Box::new(IntegerKnown {
                        rigidity: known.rigidity.clone(),
                        value: known.value.clone(),
                    }))
                    .at(source),
                ),
                ast::Integer::Generic(value) => (
                    asg::TypeKind::IntegerLiteral(value.clone()).at(source),
                    asg::Expr::new(asg::ExprKind::IntegerLiteral(value.clone()), source),
                ),
            };

            Ok(TypedExpr::new(ty, expr))
        }
        ast::ExprKind::CharLiteral(value) => {
            let expr = asg::ExprKind::IntegerKnown(Box::new(IntegerKnown {
                rigidity: IntegerRigidity::Loose(CInteger::Char, None),
                value: (*value).into(),
            }))
            .at(source);

            Ok(TypedExpr::new(
                asg::TypeKind::CInteger(CInteger::Char, None).at(source),
                expr,
            ))
        }
        ast::ExprKind::Float(value) => Ok(TypedExpr::new(
            // TODO: Clean up this code
            asg::TypeKind::FloatLiteral(NotNan::new(*value).ok()).at(source),
            asg::Expr::new(
                asg::ExprKind::FloatingLiteral(FloatSize::Bits32, NotNan::new(*value).ok()),
                source,
            ),
        )),
        ast::ExprKind::NullTerminatedString(value) => Ok(TypedExpr::new(
            asg::TypeKind::Ptr(Box::new(
                asg::TypeKind::CInteger(CInteger::Char, None).at(source),
            ))
            .at(source),
            asg::Expr::new(asg::ExprKind::NullTerminatedString(value.clone()), source),
        )),
        ast::ExprKind::String(_value) => {
            return Err(ResolveErrorKind::StringTypeNotDefined.at(source));

            // NOTE: We don't support string types yet, but once we do, we will need
            // something like this:

            /*
            let type_kind = ctx.type_search_ctx.find_type_or_error("String", source)?;

            Ok(TypedExpr::new(
                asg::TypeKind::Structure("String".into(), *struct_ref).at(source),
                asg::Expr::new(asg::ExprKind::String(value.clone()), source),
            ))
            */
        }
        ast::ExprKind::Null => Ok(TypedExpr::new(
            asg::TypeKind::Ptr(Box::new(asg::TypeKind::Void.at(source))).at(source),
            asg::Expr::new(asg::ExprKind::Null, source),
        )),
        ast::ExprKind::Call(call) => resolve_call_expr(ctx, call, source),
        ast::ExprKind::DeclareAssign(declare_assign) => {
            resolve_declare_assign_expr(ctx, declare_assign, source)
        }
        ast::ExprKind::BasicBinaryOperation(binary_operation) => {
            resolve_basic_binary_operation_expr(ctx, binary_operation, preferred_type, source)
        }
        ast::ExprKind::ShortCircuitingBinaryOperation(short_circuiting_binary_operation) => {
            resolve_short_circuiting_binary_operation_expr(
                ctx,
                short_circuiting_binary_operation,
                source,
            )
        }
        ast::ExprKind::Member(subject, field_name, min_privacy) => {
            resolve_member_expr(ctx, subject, field_name, *min_privacy, source)
        }
        ast::ExprKind::ArrayAccess(array_access) => {
            resolve_array_access_expr(ctx, array_access, source)
        }
        ast::ExprKind::StructLiteral(literal) => {
            let ast::StructLiteral {
                ast_type,
                fields,
                fill_behavior,
                language,
            } = &**literal;

            let conform_behavior = ctx.conform_behavior(*language);

            resolve_struct_literal_expr(
                ctx,
                ast_type,
                fields,
                *fill_behavior,
                conform_behavior,
                source,
            )
        }
        ast::ExprKind::UnaryOperation(unary_operation) => match &unary_operation.operator {
            UnaryOperator::Math(operator) => resolve_unary_math_operation_expr(
                ctx,
                operator,
                &unary_operation.inner,
                preferred_type,
                source,
            ),
            UnaryOperator::AddressOf => {
                let resolved_expr = resolve_expr(
                    ctx,
                    &unary_operation.inner,
                    preferred_type,
                    Initialized::Require,
                    ResolveExprMode::RequireValue,
                )?;
                let result_type = resolved_expr.ty.clone().pointer(source);
                let destination = resolve_expr_to_destination(resolved_expr)?;
                let expr = Expr::new(ExprKind::AddressOf(Box::new(destination)), source);
                return Ok(TypedExpr::new(result_type, expr));
            }
            UnaryOperator::Dereference => {
                let resolved_expr = resolve_expr(
                    ctx,
                    &unary_operation.inner,
                    preferred_type,
                    Initialized::Require,
                    ResolveExprMode::RequireValue,
                )?;

                let result_type = match &resolved_expr.ty.kind {
                    TypeKind::Ptr(inner) if !resolved_expr.ty.kind.is_ambiguous() => {
                        (**inner).clone()
                    }
                    _ => {
                        return Err(ResolveErrorKind::CannotPerformUnaryOperationForType {
                            operator: "(dereference) *".into(),
                            bad_type: resolved_expr.ty.to_string(),
                        }
                        .at(source));
                    }
                };

                return Ok(TypedExpr::new(
                    result_type,
                    Expr::new(ExprKind::Dereference(Box::new(resolved_expr)), source),
                ));
            }
        },
        ast::ExprKind::Conditional(conditional) => {
            resolve_conditional_expr(ctx, conditional, preferred_type, mode, source)
        }
        ast::ExprKind::While(while_loop) => {
            ctx.variable_haystack.begin_scope();

            let expr = resolve_expr(
                ctx,
                &while_loop.condition,
                Some(PreferredType::of(&asg::TypeKind::Boolean.at(source))),
                Initialized::Require,
                ResolveExprMode::RequireValue,
            )?;

            let condition = conform_expr_or_error(
                ctx,
                &expr,
                &asg::TypeKind::Boolean.at(source),
                ConformMode::Normal,
                ctx.adept_conform_behavior(),
                source,
            )?
            .expr;

            let block = asg::Block::new(resolve_stmts(
                ctx,
                &while_loop.block.stmts,
                ResolveExprMode::NeglectValue,
            )?);
            ctx.variable_haystack.end_scope();

            Ok(TypedExpr::new(
                asg::TypeKind::Void.at(source),
                asg::Expr::new(
                    asg::ExprKind::While(Box::new(asg::While { condition, block })),
                    source,
                ),
            ))
        }
        ast::ExprKind::Boolean(value) => Ok(TypedExpr::new(
            asg::TypeKind::Boolean.at(source),
            asg::Expr::new(asg::ExprKind::BooleanLiteral(*value), source),
        )),
        ast::ExprKind::StaticMemberValue(static_access_value) => {
            resolve_static_member_value(ctx, static_access_value)
        }
        ast::ExprKind::StaticMemberCall(static_access_call) => {
            resolve_static_member_call(ctx, static_access_call)
        }
        ast::ExprKind::SizeOf(ast_type, mode) => {
            let ty = ctx
                .type_ctx()
                .resolve(ast_type, ResolveTypeOptions::Unalias)?;

            Ok(TypedExpr::new(
                // NOTE: This will be the unsigned size integer type in the future
                // asg::TypeKind::SizeInteger(IntegerSign::Unsigned).at(source),
                asg::TypeKind::Integer(IntegerBits::Bits64, IntegerSign::Unsigned).at(source),
                asg::ExprKind::SizeOf(Box::new(ty), *mode).at(source),
            ))
        }
        ast::ExprKind::SizeOfValue(value, mode) => {
            let ty = resolve_expr(
                ctx,
                value,
                preferred_type,
                initialized,
                ResolveExprMode::RequireValue,
            )?
            .ty;

            Ok(TypedExpr::new(
                // NOTE: This will used be unsigned size integer type in the future
                // asg::TypeKind::SizeInteger(IntegerSign::Unsigned).at(source),
                asg::TypeKind::Integer(IntegerBits::Bits64, IntegerSign::Unsigned).at(source),
                asg::ExprKind::SizeOf(Box::new(ty), *mode).at(source),
            ))
        }
        ast::ExprKind::InterpreterSyscall(info) => {
            let ast::InterpreterSyscall {
                kind,
                args,
                result_type,
            } = &**info;

            let ty = ctx
                .type_ctx()
                .resolve(result_type, ResolveTypeOptions::Unalias)?;
            let mut resolved_args = Vec::with_capacity(args.len());

            for (expected_arg_type, arg) in args {
                let preferred_type = ctx
                    .type_ctx()
                    .resolve(expected_arg_type, ResolveTypeOptions::Unalias)?;

                resolved_args.push(
                    resolve_expr(
                        ctx,
                        arg,
                        Some(PreferredType::Reference(&preferred_type)),
                        Initialized::Require,
                        ResolveExprMode::RequireValue,
                    )?
                    .expr,
                );
            }

            Ok(TypedExpr::new(
                ty,
                asg::Expr::new(
                    asg::ExprKind::InterpreterSyscall(*kind, resolved_args),
                    source,
                ),
            ))
        }
        ast::ExprKind::Break => Ok(TypedExpr::new(
            asg::TypeKind::Never.at(source),
            asg::ExprKind::Break.at(source),
        )),
        ast::ExprKind::Continue => Ok(TypedExpr::new(
            asg::TypeKind::Never.at(source),
            asg::ExprKind::Continue.at(source),
        )),
        ast::ExprKind::IntegerPromote(value) => {
            // NOTE: Since this expression comes from C, there
            // should not be any untyped literals.
            let inner = resolve_expr(
                ctx,
                value,
                None,
                Initialized::AllowUninitialized,
                ResolveExprMode::RequireValue,
            )?;

            // WARNING: For now, we assume that shorts are 16 bits and ints are 32 bits
            // since this is the case for the vast majority of systems.
            // Targets where this does not hold will not be supported for now.
            // If we do wish to support them in the future,
            // We will probably need to add new types such as promoted<T> and arith<A, B>
            // to represent the possible result types on non-conformant architectures.

            let promoted_type = match &inner.ty.kind {
                TypeKind::Boolean => {
                    Some(TypeKind::CInteger(CInteger::Int, Some(IntegerSign::Signed)))
                }
                TypeKind::Integer(bits, sign) => {
                    if bits.bits() < BitUnits::of(32) {
                        Some(TypeKind::CInteger(CInteger::Int, Some(IntegerSign::Signed)))
                    } else if bits.bits() == BitUnits::of(32) {
                        Some(TypeKind::CInteger(CInteger::Int, Some(*sign)))
                    } else {
                        None
                    }
                }
                TypeKind::CInteger(c_integer, _) => match c_integer {
                    CInteger::Char | CInteger::Short => {
                        Some(TypeKind::CInteger(CInteger::Int, Some(IntegerSign::Signed)))
                    }
                    CInteger::Int | CInteger::Long | CInteger::LongLong => None,
                },
                TypeKind::SizeInteger(_) => {
                    // We will treat size integers as if they don't get promoted
                    None
                }
                TypeKind::IntegerLiteral(_) => {
                    panic!(
                        "Cannot integer promote untyped integer literal. This should never happen since untyped integer literals do not exist in C."
                    )
                }
                _ => None,
            };

            if let Some(promoted_type) = promoted_type {
                let promoted_type = promoted_type.at(ast_expr.source);

                return Ok(TypedExpr::new(
                    promoted_type.clone(),
                    ExprKind::IntegerCast(Box::new(CastFrom {
                        cast: Cast {
                            target_type: promoted_type,
                            value: inner.expr,
                        },
                        from_type: inner.ty,
                    }))
                    .at(ast_expr.source),
                ));
            }

            return Ok(inner);
        }
        ast::ExprKind::StaticAssert(condition, message) => {
            let condition = resolve_expr(
                ctx,
                condition,
                None,
                Initialized::Require,
                ResolveExprMode::RequireValue,
            )?;

            return Ok(TypedExpr::new(
                asg::TypeKind::Void.at(ast_expr.source),
                ExprKind::StaticAssert(Box::new(condition), message.clone()).at(ast_expr.source),
            ));
        }
        ast::ExprKind::Is(..) => {
            unimplemented!("legacy resolution of `is` expression");
        }
        ast::ExprKind::LabelLiteral(..) => {
            unimplemented!("legacy resolution of label expression");
        }
    }?;

    Ok(resolved_expr)
}
