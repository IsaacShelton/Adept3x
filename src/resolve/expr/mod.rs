mod array_access;
mod basic_binary_operation;
mod call;
mod conditional;
mod declare_assign;
mod member_expr;
mod short_circuiting_binary_operation;
mod struct_literal;
mod unary_operation;
mod variable;

use self::{
    array_access::resolve_array_access_expr,
    basic_binary_operation::resolve_basic_binary_operation_expr,
    short_circuiting_binary_operation::resolve_short_circuiting_binary_operation_expr,
};
use super::{
    error::ResolveError, function_search_ctx::FunctionSearchCtx,
    global_search_ctx::GlobalSearchCtx, type_search_ctx::TypeSearchCtx,
    variable_search_ctx::VariableSearchCtx, ConformMode, Initialized,
};
use crate::{
    ast::{self, ConformBehavior},
    resolve::{
        conform_expr_or_error, ensure_initialized,
        error::ResolveErrorKind,
        expr::{
            call::resolve_call_expr, conditional::resolve_conditional_expr,
            declare_assign::resolve_declare_assign_expr, member_expr::resolve_member_expr,
            struct_literal::resolve_struct_literal_expr,
            unary_operation::resolve_unary_operation_expr, variable::resolve_variable_expr,
        },
        resolve_stmts, resolve_type,
    },
    resolved::{self, FunctionRef, StructureRef, TypedExpr},
};
use ast::{FloatSize, IntegerBits, IntegerSign};
pub use basic_binary_operation::resolve_basic_binary_operator;
use indexmap::IndexMap;
use resolved::IntegerKnown;

pub struct ResolveExprCtx<'a, 'b> {
    pub resolved_ast: &'b mut resolved::Ast<'a>,
    pub function_search_ctx: &'b FunctionSearchCtx,
    pub type_search_ctx: &'b TypeSearchCtx<'a>,
    pub global_search_ctx: &'b GlobalSearchCtx,
    pub variable_search_ctx: VariableSearchCtx,
    pub resolved_function_ref: resolved::FunctionRef,
    pub helper_exprs: &'b IndexMap<String, &'a ast::HelperExpr>,
}

#[derive(Copy, Clone, Debug)]
pub enum PreferredType<'a> {
    Reference(&'a resolved::Type),
    ParameterType(FunctionRef, usize),
    ReturnType(FunctionRef),
    FieldType(StructureRef, &'a str),
}

impl<'a> PreferredType<'a> {
    pub fn of(reference: &'a resolved::Type) -> Self {
        Self::Reference(reference)
    }

    pub fn of_parameter(function_ref: FunctionRef, index: usize) -> Self {
        Self::ParameterType(function_ref, index)
    }

    pub fn view(&self, resolved_ast: &'a resolved::Ast) -> &'a resolved::Type {
        match self {
            PreferredType::Reference(reference) => reference,
            PreferredType::ParameterType(function_ref, index) => {
                &resolved_ast
                    .functions
                    .get(*function_ref)
                    .unwrap()
                    .parameters
                    .required
                    .get(*index)
                    .unwrap()
                    .resolved_type
            }
            PreferredType::ReturnType(function_ref) => {
                &resolved_ast
                    .functions
                    .get(*function_ref)
                    .unwrap()
                    .return_type
            }
            PreferredType::FieldType(structure_ref, field_name) => {
                let (_, _, field) = resolved_ast
                    .structures
                    .get(*structure_ref)
                    .expect("referenced structure to exist")
                    .fields
                    .get_full::<str>(field_name)
                    .expect("referenced struct field type to exist");

                &field.resolved_type
            }
        }
    }
}

pub fn resolve_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    ast_expr: &ast::Expr,
    preferred_type: Option<PreferredType>,
    initialized: Initialized,
) -> Result<resolved::TypedExpr, ResolveError> {
    let source = ast_expr.source;

    let resolved_expr = match &ast_expr.kind {
        ast::ExprKind::Variable(name) => {
            resolve_variable_expr(ctx, name, preferred_type, initialized, source)
        }
        ast::ExprKind::Integer(value) => {
            let (resolved_type, expr) = match value {
                ast::Integer::Known(known) => (
                    resolved::TypeKind::Integer {
                        bits: IntegerBits::from(known.bits),
                        sign: known.sign,
                    }
                    .at(source),
                    resolved::ExprKind::IntegerKnown(Box::new(IntegerKnown {
                        value: known.value.clone(),
                        bits: known.bits,
                        sign: known.sign,
                    }))
                    .at(source),
                ),
                ast::Integer::Generic(value) => (
                    resolved::TypeKind::IntegerLiteral(value.clone()).at(source),
                    resolved::Expr::new(resolved::ExprKind::IntegerLiteral(value.clone()), source),
                ),
            };

            Ok(TypedExpr::new(resolved_type, expr))
        }
        ast::ExprKind::Float(value) => Ok(TypedExpr::new(
            resolved::TypeKind::FloatLiteral(*value).at(source),
            resolved::Expr::new(resolved::ExprKind::Float(FloatSize::Normal, *value), source),
        )),
        ast::ExprKind::NullTerminatedString(value) => Ok(TypedExpr::new(
            resolved::TypeKind::Pointer(Box::new(
                resolved::TypeKind::Integer {
                    bits: IntegerBits::Bits8,
                    sign: IntegerSign::Unsigned,
                }
                .at(source),
            ))
            .at(source),
            resolved::Expr::new(
                resolved::ExprKind::NullTerminatedString(value.clone()),
                source,
            ),
        )),
        ast::ExprKind::String(value) => {
            let type_kind = ctx.type_search_ctx.find_type_or_error("String", source)?;

            let structure_ref = match type_kind {
                resolved::TypeKind::ManagedStructure(_, structure_ref) => structure_ref,
                _ => return Err(ResolveErrorKind::StringTypeNotDefined.at(source)),
            };

            Ok(TypedExpr::new(
                resolved::TypeKind::ManagedStructure("String".into(), *structure_ref).at(source),
                resolved::Expr::new(resolved::ExprKind::String(value.clone()), source),
            ))
        }
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
        ast::ExprKind::Member(subject, field_name) => {
            resolve_member_expr(ctx, subject, field_name, source)
        }
        ast::ExprKind::ArrayAccess(array_access) => {
            resolve_array_access_expr(ctx, array_access, source)
        }
        ast::ExprKind::StructureLiteral(literal) => {
            let ast::StructureLiteral {
                ast_type,
                fields,
                fill_behavior,
                conform_behavior,
            } = &**literal;

            resolve_struct_literal_expr(
                ctx,
                ast_type,
                fields,
                *fill_behavior,
                *conform_behavior,
                source,
            )
        }
        ast::ExprKind::UnaryOperation(unary_operation) => {
            resolve_unary_operation_expr(ctx, unary_operation, preferred_type, source)
        }
        ast::ExprKind::Conditional(conditional) => {
            resolve_conditional_expr(ctx, conditional, preferred_type, source)
        }
        ast::ExprKind::While(while_loop) => {
            ctx.variable_search_ctx.begin_scope();

            let condition = conform_expr_or_error(
                &resolve_expr(
                    ctx,
                    &while_loop.condition,
                    Some(PreferredType::of(&resolved::TypeKind::Boolean.at(source))),
                    Initialized::Require,
                )?,
                &resolved::TypeKind::Boolean.at(source),
                ConformMode::Normal,
                ConformBehavior::Adept,
                source,
            )?
            .expr;

            let block = resolved::Block::new(resolve_stmts(ctx, &while_loop.block.stmts)?);
            ctx.variable_search_ctx.end_scope();

            Ok(TypedExpr::new(
                resolved::TypeKind::Void.at(source),
                resolved::Expr::new(
                    resolved::ExprKind::While(Box::new(resolved::While { condition, block })),
                    source,
                ),
            ))
        }
        ast::ExprKind::Boolean(value) => Ok(TypedExpr::new(
            resolved::TypeKind::Boolean.at(source),
            resolved::Expr::new(resolved::ExprKind::BooleanLiteral(*value), source),
        )),
        ast::ExprKind::EnumMemberLiteral(enum_member_literal) => {
            let resolved_type =
                resolved::TypeKind::Enum(enum_member_literal.enum_name.clone()).at(ast_expr.source);

            Ok(TypedExpr::new(
                resolved_type,
                resolved::Expr::new(
                    resolved::ExprKind::EnumMemberLiteral(enum_member_literal.clone()),
                    source,
                ),
            ))
        }
        ast::ExprKind::InterpreterSyscall(info) => {
            let ast::InterpreterSyscall {
                kind,
                args,
                result_type,
            } = &**info;

            let resolved_type = resolve_type(
                ctx.type_search_ctx,
                ctx.resolved_ast.source_files,
                result_type,
                &mut Default::default(),
            )?;

            let mut resolved_args = Vec::with_capacity(args.len());

            for (expected_arg_type, arg) in args {
                let preferred_type = resolve_type(
                    ctx.type_search_ctx,
                    ctx.resolved_ast.source_files,
                    expected_arg_type,
                    &mut Default::default(),
                )?;

                resolved_args.push(
                    resolve_expr(
                        ctx,
                        arg,
                        Some(PreferredType::Reference(&preferred_type)),
                        Initialized::Require,
                    )?
                    .expr,
                );
            }

            Ok(TypedExpr::new(
                resolved_type,
                resolved::Expr::new(
                    resolved::ExprKind::InterpreterSyscall(*kind, resolved_args),
                    source,
                ),
            ))
        }
    }?;

    match initialized {
        Initialized::Require => {
            ensure_initialized(ast_expr, &resolved_expr)?;
        }
        Initialized::AllowUninitialized => (),
    }

    Ok(resolved_expr)
}
