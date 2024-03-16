mod error;
mod function_search_context;
mod global_search_context;
mod variable_search_context;

use crate::{
    ast::{self, Ast, FileIdentifier, Source},
    resolved::{self, TypedExpression, VariableStorage},
    source_file_cache::SourceFileCache,
};
use function_search_context::FunctionSearchContext;
use num_bigint::BigInt;
use std::collections::{HashMap, VecDeque};

use self::{
    error::{ResolveError, ResolveErrorKind},
    global_search_context::GlobalSearchContext,
    variable_search_context::VariableSearchContext,
};

enum Job {
    Regular(FileIdentifier, usize, resolved::FunctionRef),
}

#[derive(Default)]
struct ResolveContext<'a> {
    pub jobs: VecDeque<Job>,
    pub function_search_contexts: HashMap<FileIdentifier, FunctionSearchContext<'a>>,
    pub global_search_contexts: HashMap<FileIdentifier, GlobalSearchContext<'a>>,
}

pub fn resolve<'a>(ast: &'a Ast) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut ctx = ResolveContext::default();
    let mut resolved_ast = resolved::Ast::new(ast.source_file_cache);

    // Create initial jobs
    for (file_identifier, file) in ast.files.iter() {
        let global_search_context = ctx
            .global_search_contexts
            .entry(file_identifier.clone())
            .or_insert_with(|| GlobalSearchContext::new(resolved_ast.source_file_cache));

        for global in file.globals.iter() {
            let resolved_type = resolve_type(&global.ast_type)?;

            let global_ref = resolved_ast.globals.insert(resolved::Global {
                name: global.name.clone(),
                resolved_type: resolved_type.clone(),
                source: global.source,
                is_foreign: global.is_foreign,
                is_thread_local: global.is_thread_local,
            });

            global_search_context.put(global.name.to_string(), resolved_type, global_ref);
        }

        for (i, function) in file.functions.iter().enumerate() {
            let function_ref = resolved_ast.functions.insert(resolved::Function {
                name: function.name.clone(),
                parameters: resolve_parameters(&function.parameters)?,
                return_type: resolve_type(&function.return_type)?,
                statements: vec![],
                is_foreign: function.is_foreign,
                variables: VariableStorage::new(),
            });

            ctx.jobs
                .push_back(Job::Regular(file_identifier.clone(), i, function_ref));

            let function_search_context = ctx
                .function_search_contexts
                .entry(file_identifier.clone())
                .or_insert_with(|| FunctionSearchContext::new(resolved_ast.source_file_cache));

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
                let function_search_context = ctx
                    .function_search_contexts
                    .get(&file_identifier)
                    .expect("function search context to exist for file");

                let global_search_context = ctx
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

                let mut resolved_statements = vec![];

                let mut variable_search_context =
                    VariableSearchContext::new(resolved_ast.source_file_cache);

                {
                    let function = resolved_ast
                        .functions
                        .get_mut(resolved_function_ref)
                        .unwrap();

                    for parameter in ast_function.parameters.required.iter() {
                        let resolved_type = resolve_type(&parameter.ast_type)?;
                        let key = function.variables.add_parameter(resolved_type.clone());

                        variable_search_context.put(parameter.name.clone(), resolved_type, key);
                    }
                }

                for statement in ast_function.statements.iter() {
                    resolved_statements.push(resolve_statement(
                        &mut resolved_ast,
                        &function_search_context,
                        &global_search_context,
                        &mut variable_search_context,
                        resolved_function_ref,
                        statement,
                    )?);
                }

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

fn resolve_statement(
    resolved_ast: &mut resolved::Ast,
    function_search_context: &FunctionSearchContext,
    global_search_context: &GlobalSearchContext,
    variable_search_context: &mut VariableSearchContext,
    resolved_function_ref: resolved::FunctionRef,
    ast_statement: &ast::Statement,
) -> Result<resolved::Statement, ResolveError> {
    let source = ast_statement.source;

    match &ast_statement.kind {
        ast::StatementKind::Return(value) => {
            let return_value = if let Some(value) = value {
                let result = resolve_expression(
                    resolved_ast,
                    function_search_context,
                    global_search_context,
                    variable_search_context,
                    resolved_function_ref,
                    value,
                )?;

                let function = resolved_ast.functions.get(resolved_function_ref).unwrap();

                if let Some(result) = conform_expression(&result, &function.return_type) {
                    Some(result.expression)
                } else {
                    return Err(ResolveError {
                        filename: Some(
                            resolved_ast
                                .source_file_cache
                                .get(source.key)
                                .filename()
                                .to_string(),
                        ),
                        location: Some(result.expression.source.location),
                        kind: ResolveErrorKind::CannotReturnValueOfType {
                            returning: result.resolved_type.to_string(),
                            expected: function.return_type.to_string(),
                        },
                    });
                }
            } else {
                let function = resolved_ast.functions.get(resolved_function_ref).unwrap();

                if function.return_type != resolved::Type::Void {
                    return Err(ResolveError {
                        filename: Some(
                            resolved_ast
                                .source_file_cache
                                .get(source.key)
                                .filename()
                                .to_string(),
                        ),
                        location: Some(source.location),
                        kind: ResolveErrorKind::CannotReturnVoid {
                            expected: function.return_type.to_string(),
                        },
                    });
                }

                None
            };

            Ok(resolved::Statement::new(
                resolved::StatementKind::Return(return_value),
                source,
            ))
        }
        ast::StatementKind::Expression(value) => Ok(resolved::Statement::new(
            resolved::StatementKind::Expression(
                resolve_expression(
                    resolved_ast,
                    function_search_context,
                    global_search_context,
                    variable_search_context,
                    resolved_function_ref,
                    value,
                )?
                .expression,
            ),
            source,
        )),
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
            // Integer literals -> Integer
            if let Some(conformed) =
                conform_integer_literal(value, expression.expression.source, to_type)
            {
                return Some(conformed);
            }
        }
        resolved::Type::Integer { .. } => {
            // Integer -> Integer
            match to_type {
                resolved::Type::Integer {
                    bits: _new_bits,
                    sign: _new_sign,
                } => {
                    todo!();
                }
                _ => (),
            }
        }
        _ => (),
    }

    None
}

fn conform_expression_to_default(
    expression: TypedExpression,
    source_file_cache: &SourceFileCache,
) -> Result<TypedExpression, ResolveError> {
    let source = expression.expression.source;

    match expression.resolved_type {
        resolved::Type::IntegerLiteral(value) => {
            if let Some(conformed) =
                conform_integer_to_default(&value, expression.expression.source)
            {
                Ok(conformed)
            } else {
                Err(ResolveError {
                    filename: Some(source_file_cache.get(source.key).filename().to_string()),
                    location: Some(source.location),
                    kind: ResolveErrorKind::UnrepresentableInteger {
                        value: value.to_string(),
                    },
                })
            }
        }
        _ => Ok(expression),
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

fn resolve_expression(
    resolved_ast: &mut resolved::Ast,
    function_search_context: &FunctionSearchContext,
    global_search_context: &GlobalSearchContext,
    variable_search_context: &mut VariableSearchContext,
    resolved_function_ref: resolved::FunctionRef,
    ast_expression: &ast::Expression,
) -> Result<resolved::TypedExpression, ResolveError> {
    use resolved::{IntegerBits::*, IntegerSign::*};

    let source = ast_expression.source;

    match &ast_expression.kind {
        ast::ExpressionKind::Variable(name) => {
            if let Some((resolved_type, key)) = variable_search_context.find_variable(name) {
                Ok(TypedExpression::new(
                    resolved_type.clone(),
                    resolved::Expression::new(
                        resolved::ExpressionKind::Variable(resolved::Variable {
                            key: *key,
                            resolved_type: resolved_type.clone(),
                        }),
                        source,
                    ),
                ))
            } else {
                let (resolved_type, reference) =
                    global_search_context.find_global_or_error(name, source)?;

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
        ast::ExpressionKind::Integer(value) => Ok(TypedExpression::new(
            resolved::Type::IntegerLiteral(value.clone()),
            resolved::Expression::new(
                resolved::ExpressionKind::IntegerLiteral(value.clone()),
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
        ast::ExpressionKind::Call(call) => {
            let function_ref =
                function_search_context.find_function_or_error(&call.function_name, source)?;

            let function = resolved_ast.functions.get(function_ref).unwrap();
            let return_type = function.return_type.clone();

            if call.arguments.len() < function.parameters.required.len() {
                return Err(ResolveError {
                    filename: Some(
                        resolved_ast
                            .source_file_cache
                            .get(source.key)
                            .filename()
                            .to_string(),
                    ),
                    location: Some(source.location),
                    kind: ResolveErrorKind::NotEnoughArgumentsToFunction {
                        name: function.name.to_string(),
                    },
                });
            }

            if call.arguments.len() > function.parameters.required.len()
                && !function.parameters.is_cstyle_vararg
            {
                return Err(ResolveError {
                    filename: Some(
                        resolved_ast
                            .source_file_cache
                            .get(source.key)
                            .filename()
                            .to_string(),
                    ),
                    location: Some(source.location),
                    kind: ResolveErrorKind::TooManyArgumentsToFunction {
                        name: function.name.to_string(),
                    },
                });
            }

            let mut arguments = Vec::with_capacity(call.arguments.len());

            for (i, argument) in call.arguments.iter().enumerate() {
                let mut argument = resolve_expression(
                    resolved_ast,
                    function_search_context,
                    global_search_context,
                    variable_search_context,
                    resolved_function_ref,
                    argument,
                )?;

                let function = resolved_ast.functions.get(function_ref).unwrap();

                if let Some(parameter) = function.parameters.required.get(i) {
                    if let Some(conformed_argument) =
                        conform_expression(&argument, &parameter.resolved_type)
                    {
                        argument = conformed_argument;
                    } else {
                        return Err(ResolveError {
                            filename: Some(
                                resolved_ast
                                    .source_file_cache
                                    .get(source.key)
                                    .filename()
                                    .to_string(),
                            ),
                            location: Some(source.location),
                            kind: ResolveErrorKind::BadTypeForArgumentToFunction {
                                name: function.name.clone(),
                                i,
                            },
                        });
                    }
                } else {
                    match conform_expression_to_default(argument, resolved_ast.source_file_cache) {
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
        ast::ExpressionKind::DeclareAssign(declare_assign) => {
            let value = resolve_expression(
                resolved_ast,
                function_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                &declare_assign.value,
            )?;

            let value = conform_expression_to_default(value, resolved_ast.source_file_cache)?;

            let function = resolved_ast
                .functions
                .get_mut(resolved_function_ref)
                .unwrap();
            let key = function.variables.add_variable(value.resolved_type.clone());

            variable_search_context.put(&declare_assign.name, value.resolved_type.clone(), key);

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
            let mut left = resolve_expression(
                resolved_ast,
                function_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                &binary_operation.left,
            )?;

            let mut right = resolve_expression(
                resolved_ast,
                function_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                &binary_operation.right,
            )?;

            // TODO: Properly conform left and right types
            let unified_type = if left.resolved_type != right.resolved_type {
                let maybe_unified_type = match (&left.resolved_type, &right.resolved_type) {
                    (resolved::Type::IntegerLiteral(_), resolved::Type::IntegerLiteral(_)) => {
                        // TODO: We can be smarter than this
                        let unified_type = resolved::Type::Integer {
                            bits: resolved::IntegerBits::Bits64,
                            sign: resolved::IntegerSign::Signed,
                        };

                        left = conform_expression(&left, &unified_type)
                            .expect("conform left side of binary operator");
                        right = conform_expression(&right, &unified_type)
                            .expect("conform left side of binary operator");
                        Some(unified_type)
                    }
                    (a @ resolved::Type::Integer { .. }, resolved::Type::IntegerLiteral(_)) => {
                        Some(a.clone())
                    }
                    (resolved::Type::IntegerLiteral(_), b @ resolved::Type::Integer { .. }) => {
                        Some(b.clone())
                    }
                    (
                        resolved::Type::Integer {
                            bits: a_bits,
                            sign: a_sign,
                        },
                        resolved::Type::Integer {
                            bits: b_bits,
                            sign: b_sign,
                        },
                    ) if a_sign == b_sign => Some(resolved::Type::Integer {
                        bits: (*a_bits).max(*b_bits),
                        sign: *a_sign,
                    }),
                    _ => None,
                };

                match maybe_unified_type {
                    Some(unified_type) => {
                        left = conform_expression(&left, &unified_type)
                            .expect("conform left side of binary operator");
                        right = conform_expression(&right, &unified_type)
                            .expect("conform left side of binary operator");
                        unified_type
                    }
                    None => {
                        return Err(ResolveError {
                            filename: Some(
                                resolved_ast
                                    .source_file_cache
                                    .get(source.key)
                                    .filename()
                                    .to_string(),
                            ),
                            location: Some(source.location),
                            kind: ResolveErrorKind::BinaryOperatorMismatch {
                                left: left.resolved_type.to_string(),
                                right: right.resolved_type.to_string(),
                            },
                        })
                    }
                }
            } else {
                left.resolved_type.clone()
            };

            let result_type = if binary_operation.operator.returns_boolean() {
                resolved::Type::Boolean
            } else {
                unified_type
            };

            Ok(TypedExpression::new(
                result_type,
                resolved::Expression::new(
                    resolved::ExpressionKind::BinaryOperation(Box::new(
                        resolved::BinaryOperation {
                            operator: binary_operation.operator.clone(),
                            left,
                            right,
                        },
                    )),
                    source,
                ),
            ))
        }
    }
}

fn resolve_type(ast_type: &ast::Type) -> Result<resolved::Type, ResolveError> {
    match ast_type {
        ast::Type::Boolean => Ok(resolved::Type::Boolean),
        ast::Type::Integer { bits, sign } => Ok(resolved::Type::Integer {
            bits: *bits,
            sign: *sign,
        }),
        ast::Type::Pointer(inner) => Ok(resolved::Type::Pointer(Box::new(resolve_type(inner)?))),
        ast::Type::Void => Ok(resolved::Type::Void),
    }
}

fn resolve_parameters(parameters: &ast::Parameters) -> Result<resolved::Parameters, ResolveError> {
    let mut required = Vec::with_capacity(parameters.required.len());

    for parameter in parameters.required.iter() {
        required.push(resolved::Parameter {
            name: parameter.name.clone(),
            resolved_type: resolve_type(&parameter.ast_type)?,
        });
    }

    Ok(resolved::Parameters {
        required,
        is_cstyle_vararg: parameters.is_cstyle_vararg,
    })
}
