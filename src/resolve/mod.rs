use std::collections::{HashMap, VecDeque};

use crate::{
    ast::{self, Ast, File, FileIdentifier},
    error::CompilerError,
    resolved,
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
                let ast_file = ast
                    .files
                    .get(&file_identifier)
                    .expect("file referenced by job to exist");

                let ast_function = ast_file
                    .functions
                    .get(function_index)
                    .expect("function referenced by job to exist");

                let resolved_function = resolved_ast
                    .functions
                    .get_mut(resolved_function_ref)
                    .expect("resolved function head to exist");

                let search_context = ctx
                    .search_contexts
                    .get(&file_identifier)
                    .expect("search context to exist for file");

                for statement in ast_function.statements.iter() {
                    resolved_function
                        .statements
                        .push(resolve_statement(&search_context, statement)?);
                }
            }
        }
    }

    Ok(resolved_ast)
}

fn resolve_statement(
    search_context: &SearchContext,
    ast_statement: &ast::Statement,
) -> Result<resolved::Statement, CompilerError> {
    match ast_statement {
        ast::Statement::Return(value) => {
            Ok(resolved::Statement::Return(if let Some(value) = value {
                Some(resolve_expression(search_context, value)?)
            } else {
                None
            }))
        }
        ast::Statement::Expression(value) => Ok(resolved::Statement::Expression(
            resolve_expression(search_context, value)?,
        )),
    }
}

fn resolve_expression(
    search_context: &SearchContext,
    ast_expression: &ast::Expression,
) -> Result<resolved::Expression, CompilerError> {
    match ast_expression {
        ast::Expression::Variable(_) => todo!(),
        ast::Expression::Integer(value) => Ok(resolved::Expression::Integer(value.clone())),
        ast::Expression::NullTerminatedString(value) => {
            Ok(resolved::Expression::NullTerminatedString(value.clone()))
        }
        ast::Expression::Call(call) => {
            let mut arguments = Vec::with_capacity(call.arguments.len());
            
            for argument in call.arguments.iter() {
                arguments.push(resolve_expression(search_context, argument)?);
            }
            
            Ok(resolved::Expression::Call(resolved::Call {
                function: find_function_or_error(search_context, &call.function_name)?,
                arguments,
            }))
        },
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
            ast_type: resolve_type(&parameter.ast_type)?,
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
