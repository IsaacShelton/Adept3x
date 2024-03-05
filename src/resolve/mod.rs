use std::collections::{HashMap, VecDeque};

use num_bigint::BigInt;

use crate::{
    ast::{self, Ast, File, FileIdentifier, Type},
    error::CompilerError,
    resolved::{self, TypedExpression},
};

enum Job {
    Regular(FileIdentifier, usize, resolved::FunctionRef),
}

#[derive(Default)]
struct ResolveContext {
    pub jobs: VecDeque<Job>,
    pub search_contexts: HashMap<FileIdentifier, SearchContext>,
}

#[derive(Default)]
struct SearchContext {
    pub available: HashMap<String, Vec<resolved::FunctionRef>>,
}

pub fn resolve(ast: &Ast) -> Result<resolved::Ast, CompilerError> {
    let mut ctx = ResolveContext::default();
    let mut resolved_ast = resolved::Ast::default();

    // Create initial jobs
    for (file_identifier, file) in ast.files.iter() {
        for (i, function) in file.functions.iter().enumerate() {
            let function_ref = resolved_ast.functions.insert(resolved::Function {
                name: function.name.clone(),
                parameters: resolve_parameters(&function.parameters)?,
                return_type: resolve_type(&function.return_type)?,
                statements: Vec::new(),
                is_foreign: function.is_foreign,
            });

            ctx.jobs
                .push_back(Job::Regular(file_identifier.clone(), i, function_ref));

            let search_context = ctx
                .search_contexts
                .entry(file_identifier.clone())
                .or_insert_with(SearchContext::default);

            // You can blame stable rust for having to do this.
            // There is no way to "get_or_insert_mut" without pre-cloning the key.
            let function_group = match search_context.available.get_mut(&function.name) {
                Some(group) => group,
                None => {
                    search_context
                        .available
                        .insert(function.name.clone(), Vec::new());

                    search_context.available.get_mut(&function.name).unwrap()
                }
            };

            function_group.push(function_ref);
        }
    }

    // Resolve function bodies
    while let Some(job) = ctx.jobs.pop_front() {
        match job {
            Job::Regular(file_identifier, function_index, resolved_function_ref) => {
                let search_context = ctx
                    .search_contexts
                    .get(&file_identifier)
                    .expect("search context to exist for file");

                let ast_file = ast
                    .files
                    .get(&file_identifier)
                    .expect("file referenced by job to exist");

                let ast_function = ast_file
                    .functions
                    .get(function_index)
                    .expect("function referenced by job to exist");

                let mut resolved_statements = vec![];

                for statement in ast_function.statements.iter() {
                    resolved_statements.push(resolve_statement(
                        &mut resolved_ast,
                        &search_context,
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
    search_context: &SearchContext,
    resolved_function_ref: resolved::FunctionRef,
    ast_statement: &ast::Statement,
) -> Result<resolved::Statement, CompilerError> {
    match ast_statement {
        ast::Statement::Return(value) => {
            Ok(resolved::Statement::Return(if let Some(value) = value {
                let result = resolve_expression(resolved_ast, search_context, value)?;

                let function = resolved_ast.functions.get(resolved_function_ref).unwrap();

                if let Some(result) = conform_expression(&result, &function.return_type) {
                    Some(result.expression)
                } else {
                    return Err(CompilerError::during_resolve(format!(
                        "Cannot return value of type '{}', expected '{}'",
                        result.resolved_type, function.return_type,
                    )));
                }
            } else {
                let function = resolved_ast.functions.get(resolved_function_ref).unwrap();

                if function.return_type != resolved::Type::Void {
                    return Err(CompilerError::during_resolve(
                        "Cannot return void when function expects return value",
                    ));
                }

                None
            }))
        }
        ast::Statement::Expression(value) => Ok(resolved::Statement::Expression(
            resolve_expression(resolved_ast, search_context, value)?.expression,
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
            if let Some(conformed) = conform_integer(value, to_type) {
                return Some(conformed);
            }
        }
        _ => (),
    }

    None
}

fn conform_expression_to_default(
    expression: TypedExpression,
) -> Result<TypedExpression, CompilerError> {
    match expression.resolved_type {
        resolved::Type::IntegerLiteral(value) => {
            if let Some(conformed) = conform_integer_to_default(&value) {
                Ok(conformed)
            } else {
                Err(CompilerError::during_resolve(format!(
                    "Failed to lower unrepresentable integer literal {}",
                    value
                )))
            }
        }
        _ => Ok(expression),
    }
}

fn conform_integer_to_default(value: &BigInt) -> Option<TypedExpression> {
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
        if let Some(conformed) = conform_integer(value, possible_type) {
            return Some(conformed);
        }
    }

    return None;
}

fn conform_integer(value: &BigInt, to_type: &resolved::Type) -> Option<TypedExpression> {
    match to_type {
        resolved::Type::Integer { bits, sign } => {
            use resolved::{IntegerBits::*, IntegerLiteralBits, IntegerSign::*};

            let make_integer = |integer_literal_bits| {
                Some(TypedExpression::new(
                    resolved::Type::Integer {
                        bits: bits.clone(),
                        sign: sign.clone(),
                    },
                    resolved::Expression::Integer {
                        value: value.clone(),
                        bits: integer_literal_bits,
                        sign: sign.clone(),
                    },
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
    search_context: &SearchContext,
    ast_expression: &ast::Expression,
) -> Result<resolved::TypedExpression, CompilerError> {
    use resolved::{IntegerBits::*, IntegerSign::*, TypedExpression};

    match ast_expression {
        ast::Expression::Variable(_) => todo!(),
        ast::Expression::Integer(value) => Ok(TypedExpression::new(
            resolved::Type::IntegerLiteral(value.clone()),
            resolved::Expression::IntegerLiteral(value.clone()),
        )),
        ast::Expression::NullTerminatedString(value) => Ok(TypedExpression::new(
            resolved::Type::Pointer(Box::new(resolved::Type::Integer {
                bits: Bits8,
                sign: Unsigned,
            })),
            resolved::Expression::NullTerminatedString(value.clone()),
        )),
        ast::Expression::Call(call) => {
            let function_ref = find_function_or_error(search_context, &call.function_name)?;
            let function = resolved_ast.functions.get(function_ref).unwrap();
            let return_type = function.return_type.clone();

            if call.arguments.len() < function.parameters.required.len() {
                return Err(CompilerError::during_resolve(format!(
                    "Not enough arguments for call to function '{}'",
                    &function.name
                )));
            }

            if call.arguments.len() > function.parameters.required.len()
                && !function.parameters.is_cstyle_vararg
            {
                return Err(CompilerError::during_resolve(format!(
                    "Too many arguments for call to function '{}'",
                    &function.name
                )));
            }

            let mut arguments = Vec::with_capacity(call.arguments.len());

            for (i, argument) in call.arguments.iter().enumerate() {
                let mut argument = resolve_expression(resolved_ast, search_context, argument)?;

                let function = resolved_ast.functions.get(function_ref).unwrap();

                if let Some(parameter) = function.parameters.required.get(i) {
                    if let Some(conformed_argument) =
                        conform_expression(&argument, &parameter.resolved_type)
                    {
                        argument = conformed_argument;
                    } else {
                        return Err(CompilerError::during_resolve(format!(
                            "Bad type for argument #{} to function '{}'",
                            i, &function.name
                        )));
                    }
                } else {
                    match conform_expression_to_default(argument) {
                        Ok(conformed_argument) => argument = conformed_argument,
                        Err(error) => return Err(error),
                    }
                }

                arguments.push(argument.expression);
            }

            Ok(TypedExpression::new(
                return_type,
                resolved::Expression::Call(resolved::Call {
                    function: function_ref,
                    arguments,
                }),
            ))
        }
    }
}

fn resolve_type(ast_type: &ast::Type) -> Result<resolved::Type, CompilerError> {
    match ast_type {
        ast::Type::Integer { bits, sign } => Ok(resolved::Type::Integer {
            bits: bits.clone(),
            sign: sign.clone(),
        }),
        ast::Type::Pointer(inner) => Ok(resolved::Type::Pointer(Box::new(resolve_type(inner)?))),
        ast::Type::Void => Ok(resolved::Type::Void),
    }
}

fn resolve_parameters(parameters: &ast::Parameters) -> Result<resolved::Parameters, CompilerError> {
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

fn find_function_or_error(
    search_context: &SearchContext,
    name: &str,
) -> Result<resolved::FunctionRef, CompilerError> {
    match find_function(search_context, name) {
        Some(function) => Ok(function),
        None => Err(CompilerError::during_resolve(format!(
            "Failed to find function '{}'",
            name
        ))),
    }
}

fn find_function(search_context: &SearchContext, name: &str) -> Option<resolved::FunctionRef> {
    search_context
        .available
        .get(name)
        .and_then(|list| list.get(0))
        .copied()
}
