use ast::{FloatSize, IntegerBits, IntegerSign};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    ast::{self, Source},
    resolve::{
        conform_expression, conform_expression_or_error, conform_expression_to_default,
        conform_integer_to_default_or_error, ensure_initialized, error::ResolveErrorKind,
        resolve_expression_to_destination, resolve_statements, resolve_type, unify_types,
    },
    resolved::{
        self, Branch, FloatOrInteger, FloatOrSign, MemoryManagement, NumericMode, TypedExpression,
    },
};

use super::{
    error::ResolveError, function_search_ctx::FunctionSearchCtx,
    global_search_ctx::GlobalSearchCtx, type_search_ctx::TypeSearchCtx,
    variable_search_ctx::VariableSearchCtx, Initialized,
};

pub struct ResolveExpressionCtx<'a, 'b> {
    pub resolved_ast: &'b mut resolved::Ast<'a>,
    pub function_search_ctx: &'b FunctionSearchCtx<'a>,
    pub type_search_ctx: &'b TypeSearchCtx<'a>,
    pub global_search_ctx: &'b GlobalSearchCtx<'a>,
    pub variable_search_ctx: &'b mut VariableSearchCtx<'a>,
    pub resolved_function_ref: resolved::FunctionRef,
}

pub fn resolve_expression<'a>(
    ctx: &mut ResolveExpressionCtx<'_, '_>,
    ast_expression: &ast::Expression,
    initialized: Initialized,
) -> Result<resolved::TypedExpression, ResolveError> {
    use resolved::{IntegerBits::*, IntegerSign::*};

    let source = ast_expression.source;

    let resolved_expression = match &ast_expression.kind {
        ast::ExpressionKind::Variable(name) => resolve_variable_expression(ctx, name, source),
        ast::ExpressionKind::Integer(value) => Ok(TypedExpression::new(
            resolved::Type::IntegerLiteral(value.clone()),
            resolved::Expression::new(
                resolved::ExpressionKind::IntegerLiteral(value.clone()),
                source,
            ),
        )),
        ast::ExpressionKind::Float(value) => Ok(TypedExpression::new(
            resolved::Type::FloatLiteral(*value),
            resolved::Expression::new(
                resolved::ExpressionKind::Float(FloatSize::Normal, *value),
                source,
            ),
        )),
        ast::ExpressionKind::NullTerminatedString(value) => Ok(TypedExpression::new(
            resolved::Type::Pointer(Box::new(resolved::Type::Integer {
                bits: Bits8,
                sign: Unsigned,
            })),
            resolved::Expression::new(
                resolved::ExpressionKind::NullTerminatedString(value.clone()),
                source,
            ),
        )),
        ast::ExpressionKind::String(value) => {
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

            Ok(TypedExpression::new(
                resolved::Type::ManagedStructure("String".into(), *structure_ref),
                resolved::Expression::new(resolved::ExpressionKind::String(value.clone()), source),
            ))
        }
        ast::ExpressionKind::Call(call) => resolve_call_expression(ctx, call, source),
        ast::ExpressionKind::DeclareAssign(declare_assign) => {
            let value = resolve_expression(ctx, &declare_assign.value, Initialized::Require)?;

            let value = conform_expression_to_default(value, ctx.resolved_ast.source_file_cache)?;

            let function = ctx
                .resolved_ast
                .functions
                .get_mut(ctx.resolved_function_ref)
                .unwrap();

            let key = function
                .variables
                .add_variable(value.resolved_type.clone(), true);

            ctx.variable_search_ctx
                .put(&declare_assign.name, value.resolved_type.clone(), key);

            Ok(TypedExpression::new(
                value.resolved_type.clone(),
                resolved::Expression::new(
                    resolved::ExpressionKind::DeclareAssign(resolved::DeclareAssign {
                        key,
                        value: Box::new(value.expression),
                        resolved_type: value.resolved_type,
                    }),
                    source,
                ),
            ))
        }
        ast::ExpressionKind::BinaryOperation(binary_operation) => {
            resolve_binary_operation_expression(ctx, binary_operation, source)
        }
        ast::ExpressionKind::Member(subject, field_name) => {
            let resolved_subject = resolve_expression(ctx, subject, Initialized::Require)?;

            let (structure_ref, memory_management) = match resolved_subject.resolved_type {
                resolved::Type::PlainOldData(_, structure_ref) => {
                    (structure_ref, MemoryManagement::None)
                }
                resolved::Type::ManagedStructure(_, structure_ref) => {
                    (structure_ref, MemoryManagement::ReferenceCounted)
                }
                _ => {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        subject.source,
                        ResolveErrorKind::CannotGetFieldOfNonPlainOldDataType {
                            bad_type: resolved_subject.resolved_type.to_string(),
                        },
                    ))
                }
            };

            let structure = ctx
                .resolved_ast
                .structures
                .get(structure_ref)
                .expect("referenced struct to exist");

            let (index, _key, found_field) = match structure.fields.get_full(field_name) {
                Some(found) => found,
                None => {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        subject.source,
                        ResolveErrorKind::FieldDoesNotExist {
                            field_name: field_name.to_string(),
                        },
                    ))
                }
            };

            match found_field.privacy {
                resolved::Privacy::Public => (),
                resolved::Privacy::Private => {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        subject.source,
                        ResolveErrorKind::FieldIsPrivate {
                            field_name: field_name.to_string(),
                        },
                    ))
                }
            }

            let subject_destination = resolve_expression_to_destination(
                ctx.resolved_ast.source_file_cache,
                resolved_subject.expression,
            )?;

            Ok(TypedExpression::new(
                found_field.resolved_type.clone(),
                resolved::Expression::new(
                    resolved::ExpressionKind::Member(
                        subject_destination,
                        structure_ref,
                        index,
                        found_field.resolved_type.clone(),
                        memory_management,
                    ),
                    ast_expression.source,
                ),
            ))
        }
        ast::ExpressionKind::ArrayAccess(_array_access) => {
            unimplemented!("array access resolution not implemented yet!");
        }
        ast::ExpressionKind::StructureLiteral(ast_type, fields) => {
            let resolved_type = resolve_type(
                ctx.type_search_ctx,
                ctx.resolved_ast.source_file_cache,
                ast_type,
            )?;

            let (name, structure_ref, memory_management) =
                match &resolved_type {
                    resolved::Type::PlainOldData(name, structure_ref) => {
                        (name, *structure_ref, resolved::MemoryManagement::None)
                    }
                    resolved::Type::ManagedStructure(name, structure_ref) => (
                        name,
                        *structure_ref,
                        resolved::MemoryManagement::ReferenceCounted,
                    ),
                    _ => return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        ast_type.source,
                        ResolveErrorKind::CannotCreateStructLiteralForNonPlainOldDataStructure {
                            bad_type: ast_type.to_string(),
                        },
                    )),
                };

            let structure_type = resolved::Type::PlainOldData(name.clone(), structure_ref);
            let mut resolved_fields = IndexMap::new();

            for (name, value) in fields.iter() {
                let resolved_expression = resolve_expression(ctx, value, Initialized::Require)?;

                let structure = ctx
                    .resolved_ast
                    .structures
                    .get(structure_ref)
                    .expect("referenced structure to exist");

                let (index, _, field) = match structure.fields.get_full::<str>(&name) {
                    Some(field) => field,
                    None => {
                        return Err(ResolveError::new(
                            ctx.resolved_ast.source_file_cache,
                            source,
                            ResolveErrorKind::FieldDoesNotExist {
                                field_name: name.to_string(),
                            },
                        ))
                    }
                };

                let resolved_expression =
                    match conform_expression(&resolved_expression, &field.resolved_type) {
                        Some(resolved_expression) => resolved_expression,
                        None => {
                            return Err(ResolveError::new(
                                ctx.resolved_ast.source_file_cache,
                                ast_type.source,
                                ResolveErrorKind::ExpectedTypeForField {
                                    structure: ast_type.to_string(),
                                    field_name: name.to_string(),
                                    expected: field.resolved_type.to_string(),
                                },
                            ))
                        }
                    };

                resolved_fields.insert(name.to_string(), (resolved_expression.expression, index));
            }

            let structure = ctx
                .resolved_ast
                .structures
                .get(structure_ref)
                .expect("referenced structure to exist");

            if resolved_fields.len() != structure.fields.len() {
                let missing = structure
                    .fields
                    .keys()
                    .flat_map(|field_name| match resolved_fields.get(field_name) {
                        None => Some(field_name.clone()),
                        Some(_) => None,
                    })
                    .collect();

                return Err(ResolveError::new(
                    ctx.resolved_ast.source_file_cache,
                    source,
                    ResolveErrorKind::MissingFields { fields: missing },
                ));
            }

            Ok(TypedExpression::new(
                resolved_type.clone(),
                resolved::Expression::new(
                    resolved::ExpressionKind::StructureLiteral {
                        structure_type,
                        fields: resolved_fields,
                        memory_management,
                    },
                    ast_type.source,
                ),
            ))
        }
        ast::ExpressionKind::UnaryOperator(unary_operation) => {
            let resolved_expression =
                resolve_expression(ctx, &unary_operation.inner, Initialized::Require)?;

            let resolved_expression = match resolved_expression.resolved_type {
                resolved::Type::Boolean => resolved_expression,
                resolved::Type::Integer { .. } => resolved_expression,
                resolved::Type::IntegerLiteral(value) => conform_integer_to_default_or_error(
                    ctx.resolved_ast.source_file_cache,
                    &value,
                    resolved_expression.expression.source,
                )?,
                _ => {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::CannotPerformUnaryOperationForType {
                            operator: unary_operation.operator.to_string(),
                            bad_type: resolved_expression.resolved_type.to_string(),
                        },
                    ));
                }
            };

            let result_type = match unary_operation.operator {
                resolved::UnaryOperator::Not => resolved::Type::Boolean,
                resolved::UnaryOperator::BitComplement => resolved_expression.resolved_type.clone(),
                resolved::UnaryOperator::Negate => resolved_expression.resolved_type.clone(),
            };

            let expression = resolved::Expression::new(
                resolved::ExpressionKind::UnaryOperator(Box::new(resolved::UnaryOperation {
                    operator: unary_operation.operator.clone(),
                    inner: resolved_expression,
                })),
                source,
            );

            Ok(TypedExpression::new(result_type, expression))
        }
        ast::ExpressionKind::Conditional(ast::Conditional {
            conditions,
            otherwise,
        }) => {
            let mut otherwise = otherwise
                .as_ref()
                .map(|otherwise| {
                    resolve_statements(ctx, &otherwise.statements)
                        .map(|statements| resolved::Block::new(statements))
                })
                .transpose()?;

            let mut branches_without_else = Vec::with_capacity(conditions.len());

            for (expression, block) in conditions.iter() {
                let condition = resolve_expression(ctx, expression, Initialized::Require)?;

                let statements = resolve_statements(ctx, &block.statements)?;

                let condition = conform_expression_or_error(
                    ctx.resolved_ast.source_file_cache,
                    &condition,
                    &resolved::Type::Boolean,
                )?;

                let block = resolved::Block::new(statements);
                branches_without_else.push(Branch { condition, block });
            }

            let block_results = branches_without_else
                .iter()
                .map(|branch| &branch.block)
                .chain(otherwise.iter())
                .map(|block| block.get_result_type())
                .collect_vec();

            let result_type = if block_results
                .iter()
                .any(|result| result == &resolved::Type::Void)
            {
                if block_results.iter().all_equal() {
                    resolved::Type::Void
                } else {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::MismatchingYieldedTypes {
                            got: block_results
                                .iter()
                                .map(|resolved_type| resolved_type.to_string())
                                .collect_vec(),
                        },
                    ));
                }
            } else {
                let mut last_expressions = branches_without_else
                    .chunks_exact_mut(1)
                    .map(|branch| &mut branch[0].block)
                    .chain(otherwise.iter_mut())
                    .map(|block| {
                        match &mut block
                            .statements
                            .last_mut()
                            .expect("last statement to exist")
                            .kind
                        {
                            resolved::StatementKind::Expression(expression) => expression,
                            resolved::StatementKind::Return(_)
                            | resolved::StatementKind::Declaration(_)
                            | resolved::StatementKind::Assignment(_) => unreachable!(),
                        }
                    })
                    .collect_vec();

                match unify_types(&mut last_expressions[..]) {
                    Some(result_type) => result_type,
                    None => {
                        return Err(ResolveError::new(
                            ctx.resolved_ast.source_file_cache,
                            source,
                            ResolveErrorKind::MismatchingYieldedTypes {
                                got: block_results
                                    .iter()
                                    .map(|resolved_type| resolved_type.to_string())
                                    .collect_vec(),
                            },
                        ))
                    }
                }
            };

            let expression = resolved::Expression::new(
                resolved::ExpressionKind::Conditional(resolved::Conditional {
                    result_type: result_type.clone(),
                    branches: branches_without_else,
                    otherwise,
                }),
                source,
            );

            Ok(TypedExpression::new(result_type, expression))
        }
        ast::ExpressionKind::While(while_loop) => {
            let result_type = resolved::Type::Void;

            let condition = resolve_expression(ctx, &while_loop.condition, Initialized::Require)?;

            let condition = conform_expression_or_error(
                ctx.resolved_ast.source_file_cache,
                &condition,
                &resolved::Type::Boolean,
            )?
            .expression;

            let block =
                resolved::Block::new(resolve_statements(ctx, &while_loop.block.statements)?);

            let expression = resolved::Expression::new(
                resolved::ExpressionKind::While(resolved::While {
                    condition: Box::new(condition),
                    block,
                }),
                source,
            );

            Ok(TypedExpression::new(result_type, expression))
        }
        ast::ExpressionKind::Boolean(value) => Ok(TypedExpression::new(
            resolved::Type::Boolean,
            resolved::Expression::new(resolved::ExpressionKind::BooleanLiteral(*value), source),
        )),
    };

    resolved_expression.and_then(|resolved_expression| match initialized {
        Initialized::Require => {
            ensure_initialized(
                ctx.resolved_ast.source_file_cache,
                ast_expression,
                &resolved_expression,
            )?;
            Ok(resolved_expression)
        }
        Initialized::AllowUninitialized => Ok(resolved_expression),
    })
}

fn resolve_variable_expression(
    ctx: &mut ResolveExpressionCtx<'_, '_>,
    name: &str,
    source: Source,
) -> Result<TypedExpression, ResolveError> {
    if let Some((resolved_type, key)) = ctx.variable_search_ctx.find_variable(name) {
        let function = ctx
            .resolved_ast
            .functions
            .get_mut(ctx.resolved_function_ref)
            .unwrap();

        let is_initialized = function
            .variables
            .get(*key)
            .expect("found variable to exist")
            .is_initialized();

        Ok(TypedExpression::new_maybe_initialized(
            resolved_type.clone(),
            resolved::Expression::new(
                resolved::ExpressionKind::Variable(resolved::Variable {
                    key: *key,
                    resolved_type: resolved_type.clone(),
                }),
                source,
            ),
            is_initialized,
        ))
    } else {
        let (resolved_type, reference) =
            ctx.global_search_ctx.find_global_or_error(name, source)?;

        Ok(TypedExpression::new(
            resolved_type.clone(),
            resolved::Expression::new(
                resolved::ExpressionKind::GlobalVariable(resolved::GlobalVariable {
                    reference: *reference,
                    resolved_type: resolved_type.clone(),
                }),
                source,
            ),
        ))
    }
}

fn resolve_call_expression(
    ctx: &mut ResolveExpressionCtx<'_, '_>,
    call: &ast::Call,
    source: Source,
) -> Result<TypedExpression, ResolveError> {
    let function_ref = ctx
        .function_search_ctx
        .find_function_or_error(&call.function_name, source)?;

    let function = ctx.resolved_ast.functions.get(function_ref).unwrap();
    let return_type = function.return_type.clone();

    if call.arguments.len() < function.parameters.required.len() {
        return Err(ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::NotEnoughArgumentsToFunction {
                name: function.name.to_string(),
            },
        ));
    }

    if call.arguments.len() > function.parameters.required.len()
        && !function.parameters.is_cstyle_vararg
    {
        return Err(ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::TooManyArgumentsToFunction {
                name: function.name.to_string(),
            },
        ));
    }

    let mut arguments = Vec::with_capacity(call.arguments.len());

    for (i, argument) in call.arguments.iter().enumerate() {
        let mut argument = resolve_expression(ctx, argument, Initialized::Require)?;

        let function = ctx.resolved_ast.functions.get(function_ref).unwrap();

        if let Some(parameter) = function.parameters.required.get(i) {
            if let Some(conformed_argument) =
                conform_expression(&argument, &parameter.resolved_type)
            {
                argument = conformed_argument;
            } else {
                return Err(ResolveError::new(
                    ctx.resolved_ast.source_file_cache,
                    source,
                    ResolveErrorKind::BadTypeForArgumentToFunction {
                        expected: parameter.resolved_type.to_string(),
                        got: argument.resolved_type.to_string(),
                        name: function.name.clone(),
                        i,
                    },
                ));
            }
        } else {
            match conform_expression_to_default(argument, ctx.resolved_ast.source_file_cache) {
                Ok(conformed_argument) => argument = conformed_argument,
                Err(error) => return Err(error),
            }
        }

        arguments.push(argument.expression);
    }

    Ok(TypedExpression::new(
        return_type,
        resolved::Expression::new(
            resolved::ExpressionKind::Call(resolved::Call {
                function: function_ref,
                arguments,
            }),
            source,
        ),
    ))
}

fn float_or_integer_from_type(
    unified_type: &resolved::Type,
    allow_on_bools: bool,
) -> Option<FloatOrInteger> {
    match unified_type {
        resolved::Type::Boolean if allow_on_bools => Some(FloatOrInteger::Integer),
        resolved::Type::Integer { .. } => Some(FloatOrInteger::Integer),
        resolved::Type::Float(_) => Some(FloatOrInteger::Float),
        _ => None,
    }
}

fn float_or_sign_from_type(
    unified_type: &resolved::Type,
    allow_on_bools: bool,
) -> Option<FloatOrSign> {
    match unified_type {
        resolved::Type::Boolean if allow_on_bools => {
            Some(FloatOrSign::Integer(IntegerSign::Unsigned))
        }
        resolved::Type::Integer { sign, .. } => Some(FloatOrSign::Integer(*sign)),
        resolved::Type::Float(_) => Some(FloatOrSign::Float),
        _ => None,
    }
}

fn numeric_mode_from_type(unified_type: &resolved::Type) -> Option<NumericMode> {
    match unified_type {
        resolved::Type::Integer { sign, bits } => Some(match bits {
            IntegerBits::Normal => NumericMode::CheckOverflow(*sign),
            _ => NumericMode::Integer(*sign),
        }),
        resolved::Type::Float(_) => Some(NumericMode::Float),
        _ => None,
    }
}

fn resolve_binary_operation_expression(
    ctx: &mut ResolveExpressionCtx<'_, '_>,
    binary_operation: &ast::BinaryOperation,
    source: Source,
) -> Result<TypedExpression, ResolveError> {
    let mut left = resolve_expression(ctx, &binary_operation.left, Initialized::Require)?;
    let mut right = resolve_expression(ctx, &binary_operation.right, Initialized::Require)?;

    let unified_type = match unify_types(&mut [&mut left, &mut right]) {
        Some(value) => Ok(value),
        None => Err(ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::IncompatibleTypesForBinaryOperator {
                operator: binary_operation.operator.to_string(),
                left: left.resolved_type.to_string(),
                right: right.resolved_type.to_string(),
            },
        )),
    }?;

    let operator =
        match binary_operation.operator {
            ast::BinaryOperator::Add => {
                numeric_mode_from_type(&unified_type).map(resolved::BinaryOperator::Add)
            }
            ast::BinaryOperator::Subtract => {
                numeric_mode_from_type(&unified_type).map(resolved::BinaryOperator::Subtract)
            }
            ast::BinaryOperator::Multiply => {
                numeric_mode_from_type(&unified_type).map(resolved::BinaryOperator::Multiply)
            }
            ast::BinaryOperator::Divide => {
                float_or_sign_from_type(&unified_type, false).map(resolved::BinaryOperator::Divide)
            }
            ast::BinaryOperator::Modulus => {
                float_or_sign_from_type(&unified_type, false).map(resolved::BinaryOperator::Modulus)
            }
            ast::BinaryOperator::Equals => float_or_integer_from_type(&unified_type, true)
                .map(resolved::BinaryOperator::Equals),
            ast::BinaryOperator::NotEquals => float_or_integer_from_type(&unified_type, true)
                .map(resolved::BinaryOperator::NotEquals),
            ast::BinaryOperator::LessThan => float_or_sign_from_type(&unified_type, false)
                .map(resolved::BinaryOperator::LessThan),
            ast::BinaryOperator::LessThanEq => float_or_sign_from_type(&unified_type, false)
                .map(resolved::BinaryOperator::LessThanEq),
            ast::BinaryOperator::GreaterThan => float_or_sign_from_type(&unified_type, false)
                .map(resolved::BinaryOperator::GreaterThan),
            ast::BinaryOperator::GreaterThanEq => float_or_sign_from_type(&unified_type, false)
                .map(resolved::BinaryOperator::GreaterThanEq),
            ast::BinaryOperator::BitwiseAnd => matches!(
                unified_type,
                resolved::Type::Integer { .. } | resolved::Type::Boolean
            )
            .then_some(resolved::BinaryOperator::BitwiseAnd),
            ast::BinaryOperator::BitwiseOr => matches!(
                unified_type,
                resolved::Type::Integer { .. } | resolved::Type::Boolean
            )
            .then_some(resolved::BinaryOperator::BitwiseOr),
            ast::BinaryOperator::BitwiseXor => matches!(
                unified_type,
                resolved::Type::Integer { .. } | resolved::Type::Boolean
            )
            .then_some(resolved::BinaryOperator::BitwiseXor),
            ast::BinaryOperator::LeftShift => match unified_type {
                resolved::Type::Integer { sign, .. } => Some(match sign {
                    IntegerSign::Signed => resolved::BinaryOperator::LeftShift,
                    IntegerSign::Unsigned => resolved::BinaryOperator::LogicalLeftShift,
                }),
                _ => None,
            },
            ast::BinaryOperator::RightShift => match unified_type {
                resolved::Type::Integer { sign, .. } => Some(match sign {
                    IntegerSign::Signed => resolved::BinaryOperator::RightShift,
                    IntegerSign::Unsigned => resolved::BinaryOperator::LogicalRightShift,
                }),
                _ => None,
            },
            ast::BinaryOperator::LogicalLeftShift => {
                matches!(unified_type, resolved::Type::Integer { .. })
                    .then_some(resolved::BinaryOperator::BitwiseXor)
            }
            ast::BinaryOperator::LogicalRightShift => {
                matches!(unified_type, resolved::Type::Integer { .. })
                    .then_some(resolved::BinaryOperator::BitwiseXor)
            }
        };

    let operator = match operator {
        Some(operator) => operator,
        _ => {
            return Err(ResolveError::new(
                ctx.resolved_ast.source_file_cache,
                source,
                ResolveErrorKind::CannotPerformBinaryOperationForType {
                    operator: binary_operation.operator.to_string(),
                    bad_type: unified_type.to_string(),
                },
            ))
        }
    };

    let result_type = binary_operation
        .operator
        .returns_boolean()
        .then_some(resolved::Type::Boolean)
        .unwrap_or(unified_type);

    Ok(TypedExpression::new(
        result_type,
        resolved::Expression::new(
            resolved::ExpressionKind::BinaryOperation(Box::new(resolved::BinaryOperation {
                operator,
                left,
                right,
            })),
            source,
        ),
    ))
}
