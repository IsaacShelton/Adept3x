mod error;
mod expr;
mod function_search_ctx;
mod global_search_ctx;
mod type_search_ctx;
mod variable_search_ctx;

use crate::{
    ast::{self, Ast, FileIdentifier, Source},
    resolved::{self, Destination, TypedExpression, VariableStorage},
    source_file_cache::SourceFileCache,
};
use ast::{FloatSize, IntegerBits, IntegerSign};
use function_search_ctx::FunctionSearchCtx;
use indexmap::IndexMap;
use itertools::Itertools;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};
use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
};

use self::{
    error::{ResolveError, ResolveErrorKind},
    expr::{resolve_expression, ResolveExpressionCtx},
    global_search_ctx::GlobalSearchCtx,
    type_search_ctx::TypeSearchCtx,
    variable_search_ctx::VariableSearchCtx,
};

enum Job {
    Regular(FileIdentifier, usize, resolved::FunctionRef),
}

#[derive(Default)]
struct ResolveContext<'a> {
    pub jobs: VecDeque<Job>,
    pub type_search_contexts: HashMap<FileIdentifier, TypeSearchCtx<'a>>,
    pub function_search_contexts: HashMap<FileIdentifier, FunctionSearchCtx<'a>>,
    pub global_search_contexts: HashMap<FileIdentifier, GlobalSearchCtx<'a>>,
}

pub fn resolve<'a>(ast: &'a Ast) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut ctx = ResolveContext::default();
    let source_file_cache = ast.source_file_cache;
    let mut resolved_ast = resolved::Ast::new(source_file_cache);

    // Create initial jobs
    for (file_identifier, file) in ast.files.iter() {
        let type_search_context = ctx
            .type_search_contexts
            .entry(file_identifier.clone())
            .or_insert_with(|| TypeSearchCtx::new(source_file_cache));

        for structure in file.structures.iter() {
            let mut fields = IndexMap::new();

            for (field_name, field) in structure.fields.iter() {
                fields.insert(
                    field_name.into(),
                    resolved::Field {
                        resolved_type: resolve_type(
                            type_search_context,
                            source_file_cache,
                            &field.ast_type,
                        )?,
                        privacy: field.privacy,
                    },
                );
            }

            let structure_key = resolved_ast.structures.insert(resolved::Structure {
                name: structure.name.clone(),
                fields,
                is_packed: structure.is_packed,
            });

            let resolved_type =
                resolved::Type::ManagedStructure(structure.name.clone(), structure_key);

            type_search_context.put(structure.name.clone(), resolved_type);
        }

        let global_search_context = ctx
            .global_search_contexts
            .entry(file_identifier.clone())
            .or_insert_with(|| GlobalSearchCtx::new(source_file_cache));

        for global in file.globals.iter() {
            let resolved_type =
                resolve_type(type_search_context, source_file_cache, &global.ast_type)?;

            let global_ref = resolved_ast.globals.insert(resolved::Global {
                name: global.name.clone(),
                resolved_type: resolved_type.clone(),
                source: global.source,
                is_foreign: global.is_foreign,
                is_thread_local: global.is_thread_local,
            });

            global_search_context.put(global.name.clone(), resolved_type, global_ref);
        }

        for (i, function) in file.functions.iter().enumerate() {
            let function_ref = resolved_ast.functions.insert(resolved::Function {
                name: function.name.clone(),
                parameters: resolve_parameters(
                    type_search_context,
                    source_file_cache,
                    &function.parameters,
                )?,
                return_type: resolve_type(
                    type_search_context,
                    source_file_cache,
                    &function.return_type,
                )?,
                statements: vec![],
                is_foreign: function.is_foreign,
                variables: VariableStorage::new(),
            });

            ctx.jobs
                .push_back(Job::Regular(file_identifier.clone(), i, function_ref));

            let function_search_context = ctx
                .function_search_contexts
                .entry(file_identifier.clone())
                .or_insert_with(|| FunctionSearchCtx::new(source_file_cache));

            // You can blame stable rust for having to do this.
            // There is no way to "get_or_insert_mut" without pre-cloning the key.
            let function_group = match function_search_context.available.get_mut(&function.name) {
                Some(group) => group,
                None => {
                    function_search_context
                        .available
                        .insert(function.name.clone(), Vec::new());

                    function_search_context
                        .available
                        .get_mut(&function.name)
                        .unwrap()
                }
            };

            function_group.push(function_ref);
        }
    }

    // Resolve function bodies
    while let Some(job) = ctx.jobs.pop_front() {
        match job {
            Job::Regular(file_identifier, function_index, resolved_function_ref) => {
                let function_search_ctx = ctx
                    .function_search_contexts
                    .get(&file_identifier)
                    .expect("function search context to exist for file");

                let type_search_ctx = ctx
                    .type_search_contexts
                    .get(&file_identifier)
                    .expect("type search context to exist for file");

                let global_search_ctx = ctx
                    .global_search_contexts
                    .get(&file_identifier)
                    .expect("global search context to exist for file");

                let ast_file = ast
                    .files
                    .get(&file_identifier)
                    .expect("file referenced by job to exist");

                let ast_function = ast_file
                    .functions
                    .get(function_index)
                    .expect("function referenced by job to exist");

                let mut variable_search_ctx = VariableSearchCtx::new(source_file_cache);

                {
                    let function = resolved_ast
                        .functions
                        .get_mut(resolved_function_ref)
                        .unwrap();

                    for parameter in ast_function.parameters.required.iter() {
                        let resolved_type =
                            resolve_type(type_search_ctx, source_file_cache, &parameter.ast_type)?;
                        let key = function.variables.add_parameter(resolved_type.clone());

                        variable_search_ctx.put(parameter.name.clone(), resolved_type, key);
                    }
                }

                let resolved_statements = {
                    let mut ctx = ResolveExpressionCtx {
                        resolved_ast: &mut resolved_ast,
                        function_search_ctx,
                        type_search_ctx,
                        global_search_ctx,
                        variable_search_ctx: &mut variable_search_ctx,
                        resolved_function_ref,
                    };

                    resolve_statements(&mut ctx, &ast_function.statements)?
                };

                let resolved_function = resolved_ast
                    .functions
                    .get_mut(resolved_function_ref)
                    .expect("resolved function head to exist");

                resolved_function.statements = resolved_statements;
            }
        }
    }

    Ok(resolved_ast)
}

fn resolve_statements(
    ctx: &mut ResolveExpressionCtx<'_, '_>,
    statements: &[ast::Statement],
) -> Result<Vec<resolved::Statement>, ResolveError> {
    let mut resolved_statements = Vec::with_capacity(statements.len());

    for statement in statements.iter() {
        resolved_statements.push(resolve_statement(ctx, statement)?);
    }

    Ok(resolved_statements)
}

fn resolve_statement<'a>(
    ctx: &mut ResolveExpressionCtx<'_, '_>,
    ast_statement: &ast::Statement,
) -> Result<resolved::Statement, ResolveError> {
    let source = ast_statement.source;

    match &ast_statement.kind {
        ast::StatementKind::Return(value) => {
            let return_value = if let Some(value) = value {
                let result = resolve_expression(ctx, value, Initialized::Require)?;

                let function = ctx
                    .resolved_ast
                    .functions
                    .get(ctx.resolved_function_ref)
                    .unwrap();

                if let Some(result) = conform_expression(&result, &function.return_type) {
                    Some(result.expression)
                } else {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::CannotReturnValueOfType {
                            returning: result.resolved_type.to_string(),
                            expected: function.return_type.to_string(),
                        },
                    ));
                }
            } else {
                let function = ctx
                    .resolved_ast
                    .functions
                    .get(ctx.resolved_function_ref)
                    .unwrap();

                if function.return_type != resolved::Type::Void {
                    return Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::CannotReturnVoid {
                            expected: function.return_type.to_string(),
                        },
                    ));
                }

                None
            };

            Ok(resolved::Statement::new(
                resolved::StatementKind::Return(return_value),
                source,
            ))
        }
        ast::StatementKind::Expression(value) => Ok(resolved::Statement::new(
            resolved::StatementKind::Expression(resolve_expression(
                ctx,
                value,
                Initialized::Require,
            )?),
            source,
        )),
        ast::StatementKind::Declaration(declaration) => {
            let resolved_type = resolve_type(
                ctx.type_search_ctx,
                ctx.resolved_ast.source_file_cache,
                &declaration.ast_type,
            )?;

            let value = declaration
                .value
                .as_ref()
                .map(|value| resolve_expression(ctx, value, Initialized::Require))
                .transpose()?
                .as_ref()
                .map(|value| match conform_expression(value, &resolved_type) {
                    Some(value) => Ok(value.expression),
                    None => Err(ResolveError::new(
                        ctx.resolved_ast.source_file_cache,
                        source,
                        ResolveErrorKind::CannotAssignValueOfType {
                            from: value.resolved_type.to_string(),
                            to: resolved_type.to_string(),
                        },
                    )),
                })
                .transpose()?;

            let function = ctx
                .resolved_ast
                .functions
                .get_mut(ctx.resolved_function_ref)
                .unwrap();

            let key = function
                .variables
                .add_variable(resolved_type.clone(), value.is_some());

            ctx.variable_search_ctx
                .put(&declaration.name, resolved_type.clone(), key);

            Ok(resolved::Statement::new(
                resolved::StatementKind::Declaration(resolved::Declaration { key, value }),
                source,
            ))
        }
        ast::StatementKind::Assignment(assignment) => {
            let destination_expression = resolve_expression(
                ctx,
                &assignment.destination,
                Initialized::AllowUninitialized,
            )?;

            let value = resolve_expression(ctx, &assignment.value, Initialized::Require)?;

            match conform_expression(&value, &destination_expression.resolved_type) {
                Some(value) => {
                    let destination = resolve_expression_to_destination(
                        ctx.resolved_ast.source_file_cache,
                        destination_expression.expression,
                    )?;

                    // Mark destination as initialized
                    match &destination.kind {
                        resolved::DestinationKind::Variable(variable) => {
                            let function = ctx
                                .resolved_ast
                                .functions
                                .get_mut(ctx.resolved_function_ref)
                                .unwrap();

                            function
                                .variables
                                .get(variable.key)
                                .expect("variable being assigned to exists")
                                .set_initialized();
                        }
                        resolved::DestinationKind::GlobalVariable(..) => (),
                        resolved::DestinationKind::Member(..) => (),
                    }

                    Ok(resolved::Statement::new(
                        resolved::StatementKind::Assignment(resolved::Assignment {
                            destination,
                            value: value.expression,
                        }),
                        source,
                    ))
                }
                None => Err(ResolveError::new(
                    ctx.resolved_ast.source_file_cache,
                    source,
                    ResolveErrorKind::CannotAssignValueOfType {
                        from: value.resolved_type.to_string(),
                        to: destination_expression.resolved_type.to_string(),
                    },
                )),
            }
        }
    }
}

fn conform_expression_or_error(
    source_file_cache: &SourceFileCache,
    expression: &TypedExpression,
    target_type: &resolved::Type,
) -> Result<TypedExpression, ResolveError> {
    if let Some(expression) = conform_expression(expression, target_type) {
        Ok(expression)
    } else {
        Err(ResolveError::new(
            source_file_cache,
            expression.expression.source,
            ResolveErrorKind::TypeMismatch {
                left: expression.resolved_type.to_string(),
                right: target_type.to_string(),
            },
        ))
    }
}

fn conform_expression(
    expression: &TypedExpression,
    to_type: &resolved::Type,
) -> Option<TypedExpression> {
    if expression.resolved_type == *to_type {
        return Some(expression.clone());
    }

    // Integer Literal to Integer Type Conversion
    match &expression.resolved_type {
        resolved::Type::IntegerLiteral(value) => {
            // Integer literals -> Integer/Float
            conform_integer_literal(value, expression.expression.source, to_type)
        }
        resolved::Type::Integer {
            bits: from_bits,
            sign: from_sign,
        } => {
            // Integer -> Integer
            match to_type {
                resolved::Type::Integer { bits, sign } => conform_integer_value(
                    &expression.expression,
                    *from_bits,
                    *from_sign,
                    *bits,
                    *sign,
                ),
                _ => None,
            }
        }
        resolved::Type::FloatLiteral(value) => {
            // Float literals -> Float
            match to_type {
                resolved::Type::Float(size) => Some(TypedExpression::new(
                    resolved::Type::Float(*size),
                    resolved::Expression::new(
                        resolved::ExpressionKind::Float(*size, *value),
                        expression.expression.source,
                    ),
                )),
                _ => None,
            }
        }
        resolved::Type::Float(from_size) => {
            // Float -> Float
            match to_type {
                resolved::Type::Float(size) => {
                    conform_float_value(&expression.expression, *from_size, *size)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn conform_float_value(
    expression: &resolved::Expression,
    from_size: FloatSize,
    to_size: FloatSize,
) -> Option<TypedExpression> {
    let result_type = resolved::Type::Float(to_size);

    let from_bits = from_size.bits();
    let to_bits = to_size.bits();

    if from_bits == to_bits {
        return Some(TypedExpression::new(result_type, expression.clone()));
    }

    if from_bits < to_bits {
        return Some(TypedExpression::new(
            result_type.clone(),
            resolved::Expression {
                kind: resolved::ExpressionKind::FloatExtend(
                    Box::new(expression.clone()),
                    result_type,
                ),
                source: expression.source,
            },
        ));
    }

    None
}

fn conform_integer_value(
    expression: &resolved::Expression,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
) -> Option<TypedExpression> {
    if from_sign != to_sign && !(to_bits > from_bits) {
        return None;
    }

    if to_bits < from_bits {
        return None;
    }

    let result_type = resolved::Type::Integer {
        bits: to_bits,
        sign: to_sign,
    };

    if from_sign == to_sign && to_bits == from_bits {
        return Some(TypedExpression::new(result_type, expression.clone()));
    }

    Some(TypedExpression::new(
        result_type.clone(),
        resolved::Expression {
            kind: resolved::ExpressionKind::IntegerExtend(
                Box::new(expression.clone()),
                result_type,
            ),
            source: expression.source,
        },
    ))
}

fn conform_expression_to_default(
    expression: TypedExpression,
    source_file_cache: &SourceFileCache,
) -> Result<TypedExpression, ResolveError> {
    match expression.resolved_type {
        resolved::Type::IntegerLiteral(value) => conform_integer_to_default_or_error(
            source_file_cache,
            &value,
            expression.expression.source,
        ),
        resolved::Type::FloatLiteral(value) => Ok(conform_float_to_default(
            value,
            expression.expression.source,
        )),
        _ => Ok(expression),
    }
}

fn conform_float_to_default(value: f64, source: Source) -> TypedExpression {
    TypedExpression::new(
        resolved::Type::Float(FloatSize::Normal),
        resolved::Expression::new(
            resolved::ExpressionKind::Float(FloatSize::Normal, value),
            source,
        ),
    )
}

fn conform_integer_to_default_or_error(
    source_file_cache: &SourceFileCache,
    value: &BigInt,
    source: Source,
) -> Result<TypedExpression, ResolveError> {
    match conform_integer_to_default(&value, source) {
        Some(resolved) => Ok(resolved),
        None => Err(ResolveError::new(
            source_file_cache,
            source,
            ResolveErrorKind::UnrepresentableInteger {
                value: value.to_string(),
            },
        )),
    }
}

fn conform_integer_to_default(value: &BigInt, source: Source) -> Option<TypedExpression> {
    use resolved::{IntegerBits::*, IntegerSign::*};

    let possible_types = [
        resolved::Type::Integer {
            bits: Bits64,
            sign: Signed,
        },
        resolved::Type::Integer {
            bits: Bits64,
            sign: Unsigned,
        },
    ];

    for possible_type in possible_types.iter() {
        if let Some(conformed) = conform_integer_literal(value, source, possible_type) {
            return Some(conformed);
        }
    }

    return None;
}

fn conform_integer_literal(
    value: &BigInt,
    source: Source,
    to_type: &resolved::Type,
) -> Option<TypedExpression> {
    match to_type {
        resolved::Type::Float(size) => value.to_f64().map(|literal| {
            TypedExpression::new(
                resolved::Type::Float(*size),
                resolved::Expression::new(resolved::ExpressionKind::Float(*size, literal), source),
            )
        }),
        resolved::Type::Integer { bits, sign } => {
            use resolved::{IntegerBits::*, IntegerLiteralBits, IntegerSign::*};

            let make_integer = |integer_literal_bits| {
                Some(TypedExpression::new(
                    resolved::Type::Integer {
                        bits: *bits,
                        sign: *sign,
                    },
                    resolved::Expression::new(
                        resolved::ExpressionKind::Integer {
                            value: value.clone(),
                            bits: integer_literal_bits,
                            sign: *sign,
                        },
                        source,
                    ),
                ))
            };

            match (bits, sign) {
                (Normal, Signed) => {
                    if TryInto::<i64>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits64)
                    } else {
                        None
                    }
                }
                (Normal, Unsigned) => {
                    if TryInto::<u64>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits64)
                    } else {
                        None
                    }
                }
                (Bits8, Signed) => {
                    if TryInto::<i8>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits8)
                    } else {
                        None
                    }
                }
                (Bits8, Unsigned) => {
                    if TryInto::<u8>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits8)
                    } else {
                        None
                    }
                }
                (Bits16, Signed) => {
                    if TryInto::<i16>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits16)
                    } else {
                        None
                    }
                }
                (Bits16, Unsigned) => {
                    if TryInto::<u16>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits16)
                    } else {
                        None
                    }
                }
                (Bits32, Signed) => {
                    if TryInto::<i32>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits32)
                    } else {
                        None
                    }
                }
                (Bits32, Unsigned) => {
                    if TryInto::<u32>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits32)
                    } else {
                        None
                    }
                }
                (Bits64, Signed) => {
                    if TryInto::<i64>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits64)
                    } else {
                        None
                    }
                }
                (Bits64, Unsigned) => {
                    if TryInto::<u64>::try_into(value).is_ok() {
                        make_integer(IntegerLiteralBits::Bits64)
                    } else {
                        None
                    }
                }
            }
        }
        _ => None,
    }
}

enum Initialized {
    Require,
    AllowUninitialized,
}

fn resolve_type(
    type_search_context: &TypeSearchCtx<'_>,
    source_file_cache: &SourceFileCache,
    ast_type: &ast::Type,
) -> Result<resolved::Type, ResolveError> {
    match &ast_type.kind {
        ast::TypeKind::Boolean => Ok(resolved::Type::Boolean),
        ast::TypeKind::Integer { bits, sign } => Ok(resolved::Type::Integer {
            bits: *bits,
            sign: *sign,
        }),
        ast::TypeKind::Pointer(inner) => Ok(resolved::Type::Pointer(Box::new(resolve_type(
            type_search_context,
            source_file_cache,
            &inner,
        )?))),
        ast::TypeKind::Void => Ok(resolved::Type::Void),
        ast::TypeKind::Named(name) => type_search_context
            .find_type_or_error(&name, ast_type.source)
            .cloned(),
        ast::TypeKind::PlainOldData(inner) => match &inner.kind {
            ast::TypeKind::Named(name) => {
                let resolved_inner_type = type_search_context
                    .find_type_or_error(&name, ast_type.source)
                    .cloned()?;

                let structure_ref = match resolved_inner_type {
                    resolved::Type::ManagedStructure(_, structure_ref) => structure_ref,
                    _ => {
                        return Err(ResolveError::new(
                            source_file_cache,
                            inner.source,
                            ResolveErrorKind::CannotCreatePlainOldDataOfNonStructure {
                                bad_type: inner.to_string(),
                            },
                        ));
                    }
                };

                Ok(resolved::Type::PlainOldData(name.clone(), structure_ref))
            }
            _ => {
                return Err(ResolveError::new(
                    source_file_cache,
                    inner.source,
                    ResolveErrorKind::CannotCreatePlainOldDataOfNonStructure {
                        bad_type: inner.to_string(),
                    },
                ));
            }
        },
        ast::TypeKind::Float(size) => Ok(resolved::Type::Float(*size)),
    }
}

fn resolve_parameters(
    type_search_context: &TypeSearchCtx<'_>,
    source_file_cache: &SourceFileCache,
    parameters: &ast::Parameters,
) -> Result<resolved::Parameters, ResolveError> {
    let mut required = Vec::with_capacity(parameters.required.len());

    for parameter in parameters.required.iter() {
        required.push(resolved::Parameter {
            name: parameter.name.clone(),
            resolved_type: resolve_type(
                type_search_context,
                source_file_cache,
                &parameter.ast_type,
            )?,
        });
    }

    Ok(resolved::Parameters {
        required,
        is_cstyle_vararg: parameters.is_cstyle_vararg,
    })
}

pub fn resolve_expression_to_destination(
    source_file_cache: &SourceFileCache,
    expression: resolved::Expression,
) -> Result<Destination, ResolveError> {
    let source = expression.source;

    match TryInto::<Destination>::try_into(expression) {
        Ok(destination) => Ok(destination),
        Err(_) => Err(ResolveError::new(
            source_file_cache,
            source,
            ResolveErrorKind::CannotMutate,
        )),
    }
}

fn ensure_initialized(
    source_file_cache: &SourceFileCache,
    subject: &ast::Expression,
    resolved_subject: &TypedExpression,
) -> Result<(), ResolveError> {
    if resolved_subject.is_initialized {
        Ok(())
    } else {
        Err(ResolveError::new(
            source_file_cache,
            subject.source,
            match &subject.kind {
                ast::ExpressionKind::Variable(variable_name) => {
                    ResolveErrorKind::CannotUseUninitializedVariable {
                        variable_name: variable_name.clone(),
                    }
                }
                _ => ResolveErrorKind::CannotUseUninitializedValue,
            },
        ))
    }
}

fn unify_integer_properties(
    required_bits: Option<IntegerBits>,
    required_sign: Option<IntegerSign>,
    ty: &resolved::Type,
) -> Option<(Option<IntegerBits>, Option<IntegerSign>)> {
    let (new_bits, new_sign) = match ty {
        resolved::Type::Integer { bits, sign } => (
            match required_sign {
                Some(IntegerSign::Unsigned) if *sign == IntegerSign::Signed => {
                    // Compensate for situations like i32 + u32
                    bits.bits() as u64 + 1
                }
                _ => bits.bits() as u64,
            },
            Some(*sign),
        ),
        resolved::Type::IntegerLiteral(value) => {
            let unsigned_bits = value.bits();

            let (bits, sign) = if *value < BigInt::zero() {
                (unsigned_bits + 1, Some(IntegerSign::Signed))
            } else {
                (unsigned_bits, None)
            };

            (bits, sign)
        }
        _ => return None,
    };

    let check_overflow = match ty {
        resolved::Type::Integer {
            bits: IntegerBits::Normal,
            ..
        } => true,
        _ => required_bits == Some(IntegerBits::Normal),
    };

    let old_bits = match (required_sign, new_sign) {
        (Some(IntegerSign::Signed), Some(IntegerSign::Unsigned)) => {
            required_bits.map(|bits| bits.bits() + 1).unwrap_or(0)
        }
        _ => required_bits.map(|bits| bits.bits()).unwrap_or(0),
    };
    let old_sign = required_sign;

    let sign_kind = match (old_sign, new_sign) {
        (Some(old_sign), Some(new_sign)) => {
            if old_sign == IntegerSign::Signed || new_sign == IntegerSign::Signed {
                Some(IntegerSign::Signed)
            } else {
                Some(IntegerSign::Unsigned)
            }
        }
        (Some(old_sign), None) => Some(old_sign),
        (None, Some(new_sign)) => Some(new_sign),
        (None, None) => None,
    };

    let bits_kind = IntegerBits::new(new_bits.max(old_bits.into())).map(|bits| match bits {
        IntegerBits::Bits64 => {
            if check_overflow {
                IntegerBits::Normal
            } else {
                bits
            }
        }
        _ => bits,
    });

    bits_kind.map(|bits_kind| ((Some(bits_kind), sign_kind)))
}

fn bits_and_sign_for<'a>(
    types: &[&resolved::Type],
) -> Option<(Option<IntegerBits>, Option<IntegerSign>)> {
    types.iter().fold(Some((None, None)), |acc, ty| match acc {
        Some((maybe_bits, maybe_sign)) => unify_integer_properties(maybe_bits, maybe_sign, ty),
        None => None,
    })
}

fn unifying_type_for(expressions: &[impl Borrow<TypedExpression>]) -> Option<resolved::Type> {
    let types = expressions
        .iter()
        .map(|expression| &expression.borrow().resolved_type)
        .collect_vec();

    if types.iter().all_equal() {
        return Some(
            expressions
                .first()
                .map(|expression| expression.borrow().resolved_type.clone())
                .unwrap_or_else(|| resolved::Type::Void),
        );
    }

    // If all integer literals
    if types
        .iter()
        .all(|resolved_type| matches!(resolved_type, resolved::Type::IntegerLiteral(..)))
    {
        // TODO: We can be smarter than this
        return Some(resolved::Type::Integer {
            bits: IntegerBits::Normal,
            sign: IntegerSign::Signed,
        });
    }

    // If all (integer/float) literals
    if types.iter().all(|resolved_type| {
        matches!(
            resolved_type,
            resolved::Type::IntegerLiteral(..) | resolved::Type::FloatLiteral(..)
        )
    }) {
        return Some(resolved::Type::Float(FloatSize::Normal));
    }

    // If all integers and integer literals
    if types.iter().all(|resolved_type| {
        matches!(
            resolved_type,
            resolved::Type::IntegerLiteral(..) | resolved::Type::Integer { .. }
        )
    }) {
        let (bits, sign) = bits_and_sign_for(&types[..])?;

        let bits = bits.unwrap_or(IntegerBits::Normal);
        let sign = sign.unwrap_or(IntegerSign::Signed);

        return Some(resolved::Type::Integer { bits, sign });
    }

    None
}

fn unify_types(expressions: &mut [&mut TypedExpression]) -> Option<resolved::Type> {
    let unified_type = unifying_type_for(expressions);

    if let Some(unified_type) = &unified_type {
        for expression in expressions.iter_mut() {
            **expression = match conform_expression(&**expression, unified_type) {
                Some(conformed) => conformed,
                None => {
                    panic!(
                        "cannot conform to unified type {} for value of type {}",
                        unified_type.to_string(),
                        expression.resolved_type.to_string(),
                    );
                }
            }
        }
    }

    unified_type
}
