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
    conform::{conform_expr_or_error, ConformMode},
    destination::resolve_expr_to_destination,
    error::ResolveError,
    function_haystack::FunctionHaystack,
    variable_haystack::VariableHaystack,
    Initialized, ResolveTypeCtx,
};
use crate::{
    asg::{self, Asg, CurrentConstraints, Expr, ExprKind, FuncRef, StructRef, TypeKind, TypedExpr},
    ast::{
        self, CInteger, CIntegerAssumptions, ConformBehavior, IntegerKnown, Language, Settings,
        UnaryOperator,
    },
    resolve::{
        error::ResolveErrorKind,
        expr::{
            call::resolve_call_expr, conditional::resolve_conditional_expr,
            declare_assign::resolve_declare_assign_expr, member_expr::resolve_member_expr,
            struct_literal::resolve_struct_literal_expr,
            unary_operation::resolve_unary_math_operation_expr, variable::resolve_variable_expr,
        },
        resolve_stmts,
    },
    workspace::fs::FsNodeId,
};
use ast::FloatSize;
pub use basic_binary_operation::resolve_basic_binary_operator;
use ordered_float::NotNan;
use std::collections::HashMap;

pub struct ResolveExprCtx<'a, 'b> {
    pub asg: &'b mut Asg<'a>,
    pub function_haystack: &'b FunctionHaystack,
    pub variable_haystack: VariableHaystack,
    pub func_ref: Option<asg::FuncRef>,
    pub settings: &'b Settings,
    pub public_functions: &'b HashMap<FsNodeId, HashMap<String, Vec<asg::FuncRef>>>,
    pub types_in_modules: &'b HashMap<FsNodeId, HashMap<String, asg::TypeDecl>>,
    pub globals_in_modules: &'b HashMap<FsNodeId, HashMap<String, asg::GlobalVarDecl>>,
    pub helper_exprs_in_modules: &'b HashMap<FsNodeId, HashMap<String, asg::HelperExprDecl>>,
    pub module_fs_node_id: FsNodeId,
    pub physical_fs_node_id: FsNodeId,
    pub current_constraints: CurrentConstraints,
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

    pub fn of_parameter(function_ref: FuncRef, index: usize) -> Self {
        Self::ParameterType(function_ref, index)
    }

    pub fn view(&self, asg: &'a Asg) -> &'a asg::Type {
        match self {
            PreferredType::Reference(reference) => reference,
            PreferredType::ParameterType(function_ref, index) => {
                &asg.funcs
                    .get(*function_ref)
                    .unwrap()
                    .parameters
                    .required
                    .get(*index)
                    .unwrap()
                    .ty
            }
            PreferredType::ReturnType(function_ref) => {
                &asg.funcs.get(*function_ref).unwrap().return_type
            }
            PreferredType::FieldType(structure_ref, field_name) => {
                let (_, _, field) = asg
                    .structs
                    .get(*structure_ref)
                    .expect("referenced structure to exist")
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
) -> Result<asg::TypedExpr, ResolveError> {
    let source = ast_expr.source;

    let resolved_expr = match &ast_expr.kind {
        ast::ExprKind::Variable(name) => {
            resolve_variable_expr(ctx, name, preferred_type, initialized, source)
        }
        ast::ExprKind::Char(content) => {
            if content.len() == 1 {
                if let Some(preferred_type) = preferred_type {
                    if let TypeKind::CInteger(CInteger::Char, _) = preferred_type.view(ctx.asg).kind
                    {
                        let expr = asg::ExprKind::IntegerKnown(Box::new(IntegerKnown {
                            rigidity: ast::IntegerRigidity::Loose(CInteger::Char, None),
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
                    known.make_type(source),
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
                rigidity: ast::IntegerRigidity::Loose(CInteger::Char, None),
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
            asg::TypeKind::Pointer(Box::new(
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
                asg::TypeKind::Structure("String".into(), *structure_ref).at(source),
                asg::Expr::new(asg::ExprKind::String(value.clone()), source),
            ))
            */
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
                )?;

                let result_type = match &resolved_expr.ty.kind {
                    TypeKind::Pointer(inner) if !resolved_expr.ty.kind.is_ambiguous() => {
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
            resolve_conditional_expr(ctx, conditional, preferred_type, source)
        }
        ast::ExprKind::While(while_loop) => {
            ctx.variable_haystack.begin_scope();

            let expr = resolve_expr(
                ctx,
                &while_loop.condition,
                Some(PreferredType::of(&asg::TypeKind::Boolean.at(source))),
                Initialized::Require,
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

            let block = asg::Block::new(resolve_stmts(ctx, &while_loop.block.stmts)?);
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
        ast::ExprKind::EnumMemberLiteral(enum_member_literal) => {
            let ty = ctx.type_ctx().resolve(
                &ast::TypeKind::Named(enum_member_literal.enum_name.clone(), vec![])
                    .at(enum_member_literal.source),
            )?;

            let TypeKind::Enum(human_name, enum_ref) = &ty.kind else {
                return Err(ResolveErrorKind::StaticMemberOfTypeDoesNotExist {
                    ty: enum_member_literal.enum_name.to_string(),
                    member: enum_member_literal.variant_name.clone(),
                }
                .at(source));
            };

            Ok(TypedExpr::new(
                ty.clone(),
                asg::Expr::new(
                    asg::ExprKind::EnumMemberLiteral(Box::new(asg::EnumMemberLiteral {
                        human_name: human_name.clone(),
                        enum_ref: *enum_ref,
                        variant_name: enum_member_literal.variant_name.clone(),
                        source,
                    })),
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

            let ty = ctx.type_ctx().resolve(result_type)?;
            let mut resolved_args = Vec::with_capacity(args.len());

            for (expected_arg_type, arg) in args {
                let preferred_type = ctx.type_ctx().resolve(expected_arg_type)?;

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
                ty,
                asg::Expr::new(
                    asg::ExprKind::InterpreterSyscall(*kind, resolved_args),
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

fn ensure_initialized(
    subject: &ast::Expr,
    resolved_subject: &TypedExpr,
) -> Result<(), ResolveError> {
    if resolved_subject.is_initialized {
        Ok(())
    } else {
        Err(match &subject.kind {
            ast::ExprKind::Variable(variable_name) => {
                ResolveErrorKind::CannotUseUninitializedVariable {
                    variable_name: variable_name.to_string(),
                }
            }
            _ => ResolveErrorKind::CannotUseUninitializedValue,
        }
        .at(subject.source))
    }
}
