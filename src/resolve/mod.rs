mod function_search_context;
mod variable_search_context;

use crate::{
    ast::{self, Ast, File, FileIdentifier, Type},
    error::CompilerError,
    resolved::{self, TypedExpression, VariableStorage, VariableStorageKey},
};
use function_search_context::FunctionSearchContext;
use num_bigint::BigInt;
use std::collections::{HashMap, VecDeque};

use self::variable_search_context::VariableSearchContext;

enum Job {
    Regular(FileIdentifier, usize, resolved::FunctionRef),
}

#[derive(Default)]
struct ResolveContext {
    pub jobs: VecDeque<Job>,
    pub function_search_contexts: HashMap<FileIdentifier, FunctionSearchContext>,
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
                statements: vec![],
                is_foreign: function.is_foreign,
                variables: VariableStorage::new(),
            });

            ctx.jobs
                .push_back(Job::Regular(file_identifier.clone(), i, function_ref));

            let function_search_context = ctx
                .function_search_contexts
                .entry(file_identifier.clone())
                .or_insert_with(FunctionSearchContext::default);

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

                let mut variable_search_context = VariableSearchContext::default();

                {
                    let function = resolved_ast
                        .functions
                        .get_mut(resolved_function_ref)
                        .unwrap();

                    for (index, parameter) in ast_function.parameters.required.iter().enumerate() {
                        let resolved_type = resolve_type(&parameter.ast_type)?;
                        let key = function.variables.add_parameter(resolved_type.clone());

                        variable_search_context.put(parameter.name.clone(), resolved_type, key);
                    }
                }

                for statement in ast_function.statements.iter() {
                    resolved_statements.push(resolve_statement(
                        &mut resolved_ast,
                        &function_search_context,
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
    variable_search_context: &mut VariableSearchContext,
    resolved_function_ref: resolved::FunctionRef,
    ast_statement: &ast::Statement,
) -> Result<resolved::Statement, CompilerError> {
    match ast_statement {
        ast::Statement::Return(value) => {
            Ok(resolved::Statement::Return(if let Some(value) = value {
                let result = resolve_expression(
                    resolved_ast,
                    function_search_context,
                    variable_search_context,
                    resolved_function_ref,
                    value,
                )?;

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
            resolve_expression(
                resolved_ast,
                function_search_context,
                variable_search_context,
                resolved_function_ref,
                value,
            )?
            .expression,
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
    function_search_context: &FunctionSearchContext,
    variable_search_context: &mut VariableSearchContext,
    resolved_function_ref: resolved::FunctionRef,
    ast_expression: &ast::Expression,
) -> Result<resolved::TypedExpression, CompilerError> {
    use resolved::{IntegerBits::*, IntegerSign::*, TypedExpression};

    match ast_expression {
        ast::Expression::Variable(name) => {
            let (resolved_type, key) = variable_search_context.find_variable_or_error(name)?;

            Ok(TypedExpression::new(
                resolved_type.clone(),
                resolved::Expression::Variable(resolved::Variable {
                    key: *key,
                    resolved_type: resolved_type.clone(),
                }),
            ))
        }
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
            let function_ref =
                function_search_context.find_function_or_error(&call.function_name)?;
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
                let mut argument = resolve_expression(
                    resolved_ast,
                    function_search_context,
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
        ast::Expression::DeclareAssign(declare_assign) => {
            let value = resolve_expression(
                resolved_ast,
                function_search_context,
                variable_search_context,
                resolved_function_ref,
                &declare_assign.value,
            )?;

            let value = conform_expression_to_default(value)?;

            let function = resolved_ast
                .functions
                .get_mut(resolved_function_ref)
                .unwrap();
            let key = function.variables.add_variable(value.resolved_type.clone());

            variable_search_context.put(&declare_assign.name, value.resolved_type.clone(), key);

            Ok(TypedExpression::new(
                value.resolved_type,
                resolved::Expression::DeclareAssign(resolved::DeclareAssign {
                    key,
                    value: Box::new(value.expression),
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
