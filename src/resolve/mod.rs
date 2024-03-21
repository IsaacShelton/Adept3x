mod error;
mod function_search_context;
mod global_search_context;
mod type_search_context;
mod variable_search_context;

use crate::{
    ast::{self, Ast, FileIdentifier, Source},
    resolved::{self, Destination, TypedExpression, VariableStorage},
    source_file_cache::SourceFileCache,
};
use ast::{IntegerBits, IntegerSign};
use function_search_context::FunctionSearchContext;
use indexmap::IndexMap;
use num_bigint::BigInt;
use std::collections::{HashMap, VecDeque};

use self::{
    error::{ResolveError, ResolveErrorKind},
    global_search_context::GlobalSearchContext,
    type_search_context::TypeSearchContext,
    variable_search_context::VariableSearchContext,
};

enum Job {
    Regular(FileIdentifier, usize, resolved::FunctionRef),
}

#[derive(Default)]
struct ResolveContext<'a> {
    pub jobs: VecDeque<Job>,
    pub type_search_contexts: HashMap<FileIdentifier, TypeSearchContext<'a>>,
    pub function_search_contexts: HashMap<FileIdentifier, FunctionSearchContext<'a>>,
    pub global_search_contexts: HashMap<FileIdentifier, GlobalSearchContext<'a>>,
}

pub fn resolve<'a>(ast: &'a Ast) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut ctx = ResolveContext::default();
    let mut resolved_ast = resolved::Ast::new(ast.source_file_cache);

    // Create initial jobs
    for (file_identifier, file) in ast.files.iter() {
        let type_search_context = ctx
            .type_search_contexts
            .entry(file_identifier.clone())
            .or_insert_with(|| TypeSearchContext::new(resolved_ast.source_file_cache));

        for structure in file.structures.iter() {
            let mut fields = IndexMap::new();

            for (field_name, field) in structure.fields.iter() {
                fields.insert(
                    field_name.into(),
                    resolved::Field {
                        resolved_type: resolve_type(
                            type_search_context,
                            resolved_ast.source_file_cache,
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

            let resolved_type = resolved::Type::Structure(structure.name.clone(), structure_key);
            type_search_context.put(structure.name.clone(), resolved_type);
        }

        let global_search_context = ctx
            .global_search_contexts
            .entry(file_identifier.clone())
            .or_insert_with(|| GlobalSearchContext::new(resolved_ast.source_file_cache));

        for global in file.globals.iter() {
            let resolved_type = resolve_type(
                type_search_context,
                resolved_ast.source_file_cache,
                &global.ast_type,
            )?;

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
                    resolved_ast.source_file_cache,
                    &function.parameters,
                )?,
                return_type: resolve_type(
                    type_search_context,
                    resolved_ast.source_file_cache,
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

                let type_search_context = ctx
                    .type_search_contexts
                    .get(&file_identifier)
                    .expect("type search context to exist for file");

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
                        let resolved_type = resolve_type(
                            type_search_context,
                            resolved_ast.source_file_cache,
                            &parameter.ast_type,
                        )?;
                        let key = function.variables.add_parameter(resolved_type.clone());

                        variable_search_context.put(parameter.name.clone(), resolved_type, key);
                    }
                }

                for statement in ast_function.statements.iter() {
                    resolved_statements.push(resolve_statement(
                        &mut resolved_ast,
                        &function_search_context,
                        &type_search_context,
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
    type_search_context: &TypeSearchContext,
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
                    type_search_context,
                    global_search_context,
                    variable_search_context,
                    resolved_function_ref,
                    value,
                    Initialized::Require,
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
                    type_search_context,
                    global_search_context,
                    variable_search_context,
                    resolved_function_ref,
                    value,
                    Initialized::Require,
                )?
                .expression,
            ),
            source,
        )),
        ast::StatementKind::Declaration(declaration) => {
            let resolved_type = resolve_type(
                type_search_context,
                resolved_ast.source_file_cache,
                &declaration.ast_type,
            )?;

            let value = declaration
                .value
                .as_ref()
                .map(|value| {
                    resolve_expression(
                        resolved_ast,
                        function_search_context,
                        type_search_context,
                        global_search_context,
                        variable_search_context,
                        resolved_function_ref,
                        value,
                        Initialized::Require,
                    )
                })
                .transpose()?
                .as_ref()
                .map(|value| match conform_expression(value, &resolved_type) {
                    Some(value) => Ok(value.expression),
                    None => Err(ResolveError {
                        filename: Some(
                            resolved_ast
                                .source_file_cache
                                .get(source.key)
                                .filename()
                                .to_string(),
                        ),
                        location: Some(source.location),
                        kind: ResolveErrorKind::CannotAssignValueOfType {
                            from: value.resolved_type.to_string(),
                            to: resolved_type.to_string(),
                        },
                    }),
                })
                .transpose()?;

            let function = resolved_ast
                .functions
                .get_mut(resolved_function_ref)
                .unwrap();

            let key = function
                .variables
                .add_variable(resolved_type.clone(), value.is_some());
            variable_search_context.put(&declaration.name, resolved_type.clone(), key);

            Ok(resolved::Statement::new(
                resolved::StatementKind::Declaration(resolved::Declaration { key, value }),
                source,
            ))
        }
        ast::StatementKind::Assignment(assignment) => {
            let destination_expression = resolve_expression(
                resolved_ast,
                function_search_context,
                type_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                &assignment.destination,
                Initialized::AllowUninitialized,
            )?;

            let value = resolve_expression(
                resolved_ast,
                function_search_context,
                type_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                &assignment.value,
                Initialized::Require,
            )?;

            match conform_expression(&value, &destination_expression.resolved_type) {
                Some(value) => {
                    let destination = resolve_expression_to_destination(
                        resolved_ast.source_file_cache,
                        destination_expression.expression,
                    )?;

                    // Mark destination as initialized
                    match &destination.kind {
                        resolved::DestinationKind::Variable(variable) => {
                            let function = resolved_ast
                                .functions
                                .get_mut(resolved_function_ref)
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
                None => Err(ResolveError {
                    filename: Some(
                        resolved_ast
                            .source_file_cache
                            .get(source.key)
                            .filename()
                            .to_string(),
                    ),
                    location: Some(source.location),
                    kind: ResolveErrorKind::CannotAssignValueOfType {
                        from: value.resolved_type.to_string(),
                        to: destination_expression.resolved_type.to_string(),
                    },
                }),
            }
        }
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
        _ => None,
    }
}

fn conform_integer_value(
    expression: &resolved::Expression,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
) -> Option<TypedExpression> {
    if from_sign != to_sign {
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

enum Initialized {
    Require,
    AllowUninitialized,
}

fn resolve_expression(
    resolved_ast: &mut resolved::Ast,
    function_search_context: &FunctionSearchContext,
    type_search_context: &TypeSearchContext,
    global_search_context: &GlobalSearchContext,
    variable_search_context: &mut VariableSearchContext,
    resolved_function_ref: resolved::FunctionRef,
    ast_expression: &ast::Expression,
    initialized: Initialized,
) -> Result<resolved::TypedExpression, ResolveError> {
    use resolved::{IntegerBits::*, IntegerSign::*};

    let source = ast_expression.source;

    let resolved_expression = match &ast_expression.kind {
        ast::ExpressionKind::Variable(name) => {
            if let Some((resolved_type, key)) = variable_search_context.find_variable(name) {
                let function = resolved_ast
                    .functions
                    .get_mut(resolved_function_ref)
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
                    type_search_context,
                    global_search_context,
                    variable_search_context,
                    resolved_function_ref,
                    argument,
                    Initialized::Require,
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
                type_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                &declare_assign.value,
                Initialized::Require,
            )?;

            let value = conform_expression_to_default(value, resolved_ast.source_file_cache)?;

            let function = resolved_ast
                .functions
                .get_mut(resolved_function_ref)
                .unwrap();

            let key = function
                .variables
                .add_variable(value.resolved_type.clone(), true);
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
                type_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                &binary_operation.left,
                Initialized::Require,
            )?;

            let mut right = resolve_expression(
                resolved_ast,
                function_search_context,
                type_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                &binary_operation.right,
                Initialized::Require,
            )?;

            // TODO: Properly conform left and right types
            let unified_type = if left.resolved_type != right.resolved_type {
                let maybe_unified_type = match (&left.resolved_type, &right.resolved_type) {
                    (resolved::Type::IntegerLiteral(_), resolved::Type::IntegerLiteral(_)) => {
                        // TODO: We can be smarter than this
                        Some(resolved::Type::Integer {
                            bits: resolved::IntegerBits::Normal,
                            sign: resolved::IntegerSign::Signed,
                        })
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
                            .expect("conform right side of binary operator");
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
        ast::ExpressionKind::Member(subject, field_name) => {
            let resolved_subject = resolve_expression(
                resolved_ast,
                function_search_context,
                type_search_context,
                global_search_context,
                variable_search_context,
                resolved_function_ref,
                subject,
                Initialized::Require,
            )?;

            ensure_initialized(resolved_ast.source_file_cache, subject, &resolved_subject)?;

            let structure_ref = match resolved_subject.resolved_type {
                resolved::Type::PlainOldData(_, structure_ref) => structure_ref,
                _ => {
                    return Err(ResolveError {
                        filename: Some(
                            resolved_ast
                                .source_file_cache
                                .get(subject.source.key)
                                .filename()
                                .to_string(),
                        ),
                        location: Some(subject.source.location),
                        kind: ResolveErrorKind::CannotGetFieldOfNonPlainOldDataType {
                            bad_type: resolved_subject.resolved_type.to_string(),
                        },
                    })
                }
            };

            let structure = resolved_ast
                .structures
                .get(structure_ref)
                .expect("referenced struct to exist");

            let (index, _key, found_field) = match structure.fields.get_full(field_name) {
                Some(found) => found,
                None => {
                    return Err(ResolveError {
                        filename: Some(
                            resolved_ast
                                .source_file_cache
                                .get(subject.source.key)
                                .filename()
                                .to_string(),
                        ),
                        location: Some(subject.source.location),
                        kind: ResolveErrorKind::FieldDoesNotExist {
                            field_name: field_name.to_string(),
                        },
                    })
                }
            };

            match found_field.privacy {
                resolved::Privacy::Public => (),
                resolved::Privacy::Private => {
                    return Err(ResolveError {
                        filename: Some(
                            resolved_ast
                                .source_file_cache
                                .get(subject.source.key)
                                .filename()
                                .to_string(),
                        ),
                        location: Some(subject.source.location),
                        kind: ResolveErrorKind::FieldIsPrivate {
                            field_name: field_name.to_string(),
                        },
                    })
                }
            }

            let subject_destination = resolve_expression_to_destination(
                resolved_ast.source_file_cache,
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
                    ),
                    ast_expression.source,
                ),
            ))
        }
        ast::ExpressionKind::StructureLiteral(ast_type, fields) => {
            let resolved_type = resolve_type(
                type_search_context,
                resolved_ast.source_file_cache,
                ast_type,
            )?;

            let structure_ref =
                match resolved_type {
                    resolved::Type::PlainOldData(_, structure_ref) => structure_ref,
                    _ => return Err(ResolveError {
                        filename: Some(
                            resolved_ast
                                .source_file_cache
                                .get(ast_type.source.key)
                                .filename()
                                .to_string(),
                        ),
                        location: Some(ast_type.source.location),
                        kind:
                            ResolveErrorKind::CannotCreateStructLiteralForNonPlainOldDataStructure {
                                bad_type: ast_type.to_string(),
                            },
                    }),
                };

            let mut resolved_fields = IndexMap::new();

            for (name, value) in fields.iter() {
                let resolved_expression = resolve_expression(
                    resolved_ast,
                    function_search_context,
                    type_search_context,
                    global_search_context,
                    variable_search_context,
                    resolved_function_ref,
                    value,
                    Initialized::Require,
                )?;

                let structure = resolved_ast
                    .structures
                    .get(structure_ref)
                    .expect("referenced structure to exist");

                let (index, _, field) = match structure.fields.get_full::<str>(&name) {
                    Some(field) => field,
                    None => {
                        return Err(ResolveError {
                            filename: Some(
                                resolved_ast
                                    .source_file_cache
                                    .get(ast_type.source.key)
                                    .filename()
                                    .to_string(),
                            ),
                            location: Some(ast_type.source.location),
                            kind: ResolveErrorKind::FieldDoesNotExist {
                                field_name: name.to_string(),
                            },
                        })
                    }
                };

                let resolved_expression =
                    match conform_expression(&resolved_expression, &field.resolved_type) {
                        Some(resolved_expression) => resolved_expression,
                        None => {
                            return Err(ResolveError {
                                filename: Some(
                                    resolved_ast
                                        .source_file_cache
                                        .get(ast_type.source.key)
                                        .filename()
                                        .to_string(),
                                ),
                                location: Some(ast_type.source.location),
                                kind: ResolveErrorKind::FieldDoesNotExist {
                                    field_name: name.to_string(),
                                },
                            })
                        }
                    };

                resolved_fields.insert(name.to_string(), (resolved_expression.expression, index));
            }

            let structure = resolved_ast
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

                return Err(ResolveError {
                    filename: Some(
                        resolved_ast
                            .source_file_cache
                            .get(ast_type.source.key)
                            .filename()
                            .to_string(),
                    ),
                    location: Some(ast_type.source.location),
                    kind: ResolveErrorKind::MissingFields { fields: missing },
                });
            }

            Ok(TypedExpression::new(
                resolved_type.clone(),
                resolved::Expression::new(
                    resolved::ExpressionKind::StructureLiteral(resolved_type, resolved_fields),
                    ast_type.source,
                ),
            ))
        }
    };

    resolved_expression.and_then(|resolved_expression| match initialized {
        Initialized::Require => {
            ensure_initialized(
                resolved_ast.source_file_cache,
                ast_expression,
                &resolved_expression,
            )?;
            Ok(resolved_expression)
        }
        Initialized::AllowUninitialized => Ok(resolved_expression),
    })
}

fn resolve_type(
    type_search_context: &TypeSearchContext<'_>,
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
                    resolved::Type::Structure(_, structure_ref) => structure_ref,
                    _ => {
                        return Err(ResolveError {
                            filename: Some(
                                source_file_cache
                                    .get(inner.source.key)
                                    .filename()
                                    .to_string(),
                            ),
                            location: Some(inner.source.location),
                            kind: ResolveErrorKind::CannotCreatePlainOldDataOfNonStructure {
                                bad_type: inner.to_string(),
                            },
                        })
                    }
                };

                Ok(resolved::Type::PlainOldData(name.clone(), structure_ref))
            }
            _ => {
                return Err(ResolveError {
                    filename: Some(
                        source_file_cache
                            .get(inner.source.key)
                            .filename()
                            .to_string(),
                    ),
                    location: Some(inner.source.location),
                    kind: ResolveErrorKind::CannotCreatePlainOldDataOfNonStructure {
                        bad_type: inner.to_string(),
                    },
                })
            }
        },
    }
}

fn resolve_parameters(
    type_search_context: &TypeSearchContext<'_>,
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
        Err(_) => {
            return Err(ResolveError {
                filename: Some(source_file_cache.get(source.key).filename().to_string()),
                location: Some(source.location),
                kind: ResolveErrorKind::CannotMutate,
            })
        }
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
        let resolve_error_kind = match &subject.kind {
            ast::ExpressionKind::Variable(variable_name) => {
                ResolveErrorKind::CannotUseUninitializedVariable {
                    variable_name: variable_name.clone(),
                }
            }
            _ => ResolveErrorKind::CannotUseUninitializedValue,
        };

        return Err(ResolveError {
            filename: Some(
                source_file_cache
                    .get(subject.source.key)
                    .filename()
                    .to_string(),
            ),
            location: Some(subject.source.location),
            kind: resolve_error_kind,
        });
    }
}
