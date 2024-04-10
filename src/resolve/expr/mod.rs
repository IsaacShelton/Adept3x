mod binary_operation;
mod call;
mod conditional;
mod declare_assign;
mod member_expr;
mod struct_literal;
mod unary_operation;
mod variable;

use super::{
    error::ResolveError, function_search_ctx::FunctionSearchCtx,
    global_search_ctx::GlobalSearchCtx, type_search_ctx::TypeSearchCtx,
    variable_search_ctx::VariableSearchCtx, Initialized,
};
use crate::{
    ast::{self},
    resolve::{
        conform_expr_or_error, ensure_initialized,
        error::ResolveErrorKind,
        expr::{
            binary_operation::resolve_binary_operation_expr, call::resolve_call_expr,
            conditional::resolve_conditional_expr, declare_assign::resolve_declare_assign_expr,
            member_expr::resolve_member_expr, struct_literal::resolve_struct_literal_expr,
            unary_operation::resolve_unary_operation_expr, variable::resolve_variable_expr,
        },
        resolve_stmts,
    },
    resolved::{self, TypedExpr},
};
use ast::{FloatSize, IntegerBits, IntegerSign};

pub struct ResolveExprCtx<'a, 'b> {
    pub resolved_ast: &'b mut resolved::Ast<'a>,
    pub function_search_ctx: &'b FunctionSearchCtx<'a>,
    pub type_search_ctx: &'b TypeSearchCtx<'a>,
    pub global_search_ctx: &'b GlobalSearchCtx<'a>,
    pub variable_search_ctx: &'b mut VariableSearchCtx<'a>,
    pub resolved_function_ref: resolved::FunctionRef,
}

pub fn resolve_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    ast_expr: &ast::Expr,
    initialized: Initialized,
) -> Result<resolved::TypedExpr, ResolveError> {
    let source = ast_expr.source;

    let resolved_expr = match &ast_expr.kind {
        ast::ExprKind::Variable(name) => resolve_variable_expr(ctx, name, source),
        ast::ExprKind::Integer(value) => Ok(TypedExpr::new(
            resolved::Type::IntegerLiteral(value.clone()),
            resolved::Expr::new(resolved::ExprKind::IntegerLiteral(value.clone()), source),
        )),
        ast::ExprKind::Float(value) => Ok(TypedExpr::new(
            resolved::Type::FloatLiteral(*value),
            resolved::Expr::new(resolved::ExprKind::Float(FloatSize::Normal, *value), source),
        )),
        ast::ExprKind::NullTerminatedString(value) => Ok(TypedExpr::new(
            resolved::Type::Pointer(Box::new(resolved::Type::Integer {
                bits: IntegerBits::Bits8,
                sign: IntegerSign::Unsigned,
            })),
            resolved::Expr::new(
                resolved::ExprKind::NullTerminatedString(value.clone()),
                source,
            ),
        )),
        ast::ExprKind::String(value) => {
            let resolved_type = ctx.type_search_ctx.find_type_or_error("String", source)?;

            let structure_ref = match resolved_type {
                resolved::Type::ManagedStructure(_, structure_ref) => structure_ref,
                _ => {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::StringTypeNotDefined,
                    ))
                }
            };

            Ok(TypedExpr::new(
                resolved::Type::ManagedStructure("String".into(), *structure_ref),
                resolved::Expr::new(resolved::ExprKind::String(value.clone()), source),
            ))
        }
        ast::ExprKind::Call(call) => resolve_call_expr(ctx, call, source),
        ast::ExprKind::DeclareAssign(declare_assign) => {
            resolve_declare_assign_expr(ctx, declare_assign, source)
        }
        ast::ExprKind::BinaryOperation(binary_operation) => {
            resolve_binary_operation_expr(ctx, binary_operation, source)
        }
        ast::ExprKind::Member(subject, field_name) => {
            resolve_member_expr(ctx, subject, field_name, source)
        }
        ast::ExprKind::ArrayAccess(_array_access) => {
            unimplemented!("array access resolution not implemented yet!");
        }
        ast::ExprKind::StructureLiteral(ast_type, fields) => {
            resolve_struct_literal_expr(ctx, ast_type, fields, source)
        }
        ast::ExprKind::UnaryOperation(unary_operation) => {
            resolve_unary_operation_expr(ctx, unary_operation, source)
        }
        ast::ExprKind::Conditional(conditional) => {
            resolve_conditional_expr(ctx, conditional, source)
        }
        ast::ExprKind::While(while_loop) => {
            let condition = conform_expr_or_error(
                ctx.resolved_ast.source_file_cache,
                &resolve_expr(ctx, &while_loop.condition, Initialized::Require)?,
                &resolved::Type::Boolean,
            )?
            .expr;

            let block = resolved::Block::new(resolve_stmts(ctx, &while_loop.block.stmts)?);

            Ok(TypedExpr::new(
                resolved::Type::Void,
                resolved::Expr::new(
                    resolved::ExprKind::While(resolved::While {
                        condition: Box::new(condition),
                        block,
                    }),
                    source,
                ),
            ))
        }
        ast::ExprKind::Boolean(value) => Ok(TypedExpr::new(
            resolved::Type::Boolean,
            resolved::Expr::new(resolved::ExprKind::BooleanLiteral(*value), source),
        )),
    }?;

    match initialized {
        Initialized::Require => {
            ensure_initialized(ctx.resolved_ast.source_file_cache, ast_expr, &resolved_expr)?;
        }
        Initialized::AllowUninitialized => (),
    }

    Ok(resolved_expr)
}
