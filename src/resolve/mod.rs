mod core_structure_info;
mod destination;
mod error;
mod expr;
mod function_search_ctx;
mod global_search_ctx;
mod lifetime;
mod stmt;
mod type_search_ctx;
mod unify_types;
mod variable_search_ctx;

use self::{
    error::{ResolveError, ResolveErrorKind},
    expr::ResolveExprCtx,
    global_search_ctx::GlobalSearchCtx,
    stmt::resolve_stmts,
    type_search_ctx::TypeSearchCtx,
    variable_search_ctx::VariableSearchCtx,
};
use crate::{
    ast::{self, Ast, FileIdentifier, Source},
    resolved::{self, TypedExpr, VariableStorage},
    source_file_cache::SourceFileCache,
};
use ast::{FloatSize, IntegerBits, IntegerSign};
use function_search_ctx::FunctionSearchCtx;
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::{HashMap, VecDeque};

enum Job {
    Regular(FileIdentifier, usize, resolved::FunctionRef),
}

#[derive(Default)]
struct ResolveCtx<'a> {
    pub jobs: VecDeque<Job>,
    pub type_search_contexts: HashMap<String, TypeSearchCtx<'a>>,
    pub function_search_contexts: HashMap<String, FunctionSearchCtx<'a>>,
    pub global_search_contexts: HashMap<String, GlobalSearchCtx<'a>>,
}

pub fn resolve<'a>(ast: &'a Ast) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut ctx = ResolveCtx::default();
    let source_file_cache = ast.source_file_cache;
    let mut resolved_ast = resolved::Ast::new(source_file_cache);

    // Create initial jobs
    for (file_identifier, file) in ast.files.iter() {
        let type_search_ctx = ctx
            .type_search_contexts
            .entry(ast.primary_filename.clone())
            .or_insert_with(|| TypeSearchCtx::new(source_file_cache));

        for structure in file.structures.iter() {
            let mut fields = IndexMap::new();

            for (field_name, field) in structure.fields.iter() {
                fields.insert(
                    field_name.into(),
                    resolved::Field {
                        resolved_type: resolve_type(
                            type_search_ctx,
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

            if structure.prefer_pod {
                type_search_ctx.put(
                    structure.name.clone(),
                    resolved::TypeKind::PlainOldData(structure.name.clone(), structure_key),
                );
            } else {
                type_search_ctx.put(
                    structure.name.clone(),
                    resolved::TypeKind::ManagedStructure(structure.name.clone(), structure_key),
                );
            }
        }

        let global_search_context = ctx
            .global_search_contexts
            .entry(ast.primary_filename.clone())
            .or_insert_with(|| GlobalSearchCtx::new(source_file_cache));

        for global in file.globals.iter() {
            let resolved_type = resolve_type(type_search_ctx, source_file_cache, &global.ast_type)?;

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
                    type_search_ctx,
                    source_file_cache,
                    &function.parameters,
                )?,
                return_type: resolve_type(
                    type_search_ctx,
                    source_file_cache,
                    &function.return_type,
                )?,
                stmts: vec![],
                is_foreign: function.is_foreign,
                variables: VariableStorage::new(),
                source: function.source,
            });

            ctx.jobs
                .push_back(Job::Regular(file_identifier.clone(), i, function_ref));

            let function_search_context = ctx
                .function_search_contexts
                .entry(ast.primary_filename.clone())
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
                    .get(&ast.primary_filename)
                    .expect("function search context to exist for file");

                let type_search_ctx = ctx
                    .type_search_contexts
                    .get(&ast.primary_filename)
                    .expect("type search context to exist for file");

                let global_search_ctx = ctx
                    .global_search_contexts
                    .get(&ast.primary_filename)
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

                let resolved_stmts = {
                    let mut ctx = ResolveExprCtx {
                        resolved_ast: &mut resolved_ast,
                        function_search_ctx,
                        type_search_ctx,
                        global_search_ctx,
                        variable_search_ctx,
                        resolved_function_ref,
                    };

                    resolve_stmts(&mut ctx, &ast_function.stmts)?
                };

                let resolved_function = resolved_ast
                    .functions
                    .get_mut(resolved_function_ref)
                    .expect("resolved function head to exist");

                resolved_function.stmts = resolved_stmts;

                lifetime::insert_drops(resolved_function);
            }
        }
    }

    Ok(resolved_ast)
}

fn conform_expr_or_error(
    expr: &TypedExpr,
    target_type: &resolved::Type,
    mode: ConformMode,
    conform_source: Source,
) -> Result<TypedExpr, ResolveError> {
    if let Some(expr) = conform_expr(expr, target_type, mode, conform_source) {
        Ok(expr)
    } else {
        Err(ResolveErrorKind::TypeMismatch {
            left: expr.resolved_type.to_string(),
            right: target_type.to_string(),
        }
        .at(expr.expr.source))
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub enum ConformMode {
    #[default]
    Normal,
    ParameterPassing,
    Explicit,
}

impl ConformMode {
    pub fn allow_pointer_into_void_pointer(&self) -> bool {
        match self {
            Self::Normal => false,
            Self::ParameterPassing => true,
            Self::Explicit => true,
        }
    }
}

fn conform_expr(
    expr: &TypedExpr,
    to_type: &resolved::Type,
    mode: ConformMode,
    conform_source: Source,
) -> Option<TypedExpr> {
    if expr.resolved_type == *to_type {
        return Some(expr.clone());
    }

    match &expr.resolved_type.kind {
        resolved::TypeKind::IntegerLiteral(value) => {
            // Integer literal Conversion
            conform_integer_literal(value, expr.expr.source, to_type)
        }
        resolved::TypeKind::Integer {
            bits: from_bits,
            sign: from_sign,
        } => {
            // Integer Conversion
            match &to_type.kind {
                resolved::TypeKind::Integer { bits, sign } => conform_integer_value(
                    &expr.expr,
                    *from_bits,
                    *from_sign,
                    *bits,
                    *sign,
                    to_type.source,
                ),
                _ => None,
            }
        }
        resolved::TypeKind::FloatLiteral(value) => {
            // Float Literal Conversion
            match &to_type.kind {
                resolved::TypeKind::Float(size) => Some(TypedExpr::new(
                    resolved::TypeKind::Float(*size).at(to_type.source),
                    resolved::Expr::new(resolved::ExprKind::Float(*size, *value), conform_source),
                )),
                _ => None,
            }
        }
        resolved::TypeKind::Float(from_size) => {
            // Float Conversion
            match &to_type.kind {
                resolved::TypeKind::Float(size) => {
                    conform_float_value(&expr.expr, *from_size, *size, to_type.source)
                }
                _ => None,
            }
        }
        resolved::TypeKind::Pointer(inner) => {
            // Pointer Conversion
            if inner.kind.is_void() {
                // ptr<void> -> ptr<ANYTHING>
                match &to_type.kind {
                    resolved::TypeKind::Pointer(inner) => Some(TypedExpr::new(
                        resolved::TypeKind::Pointer(inner.clone()).at(to_type.source),
                        expr.expr.clone(),
                    )),
                    _ => None,
                }
            } else if mode.allow_pointer_into_void_pointer() && to_type.kind.is_void_pointer() {
                // ptr<ANYTHING> -> ptr<void>
                Some(TypedExpr::new(
                    resolved::TypeKind::Pointer(Box::new(
                        resolved::TypeKind::Void.at(to_type.source),
                    ))
                    .at(to_type.source),
                    expr.expr.clone(),
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn conform_float_value(
    expr: &resolved::Expr,
    from_size: FloatSize,
    to_size: FloatSize,
    type_source: Source,
) -> Option<TypedExpr> {
    let result_type = resolved::TypeKind::Float(to_size).at(type_source);

    let from_bits = from_size.bits();
    let to_bits = to_size.bits();

    if from_bits == to_bits {
        return Some(TypedExpr::new(result_type, expr.clone()));
    }

    if from_bits < to_bits {
        return Some(TypedExpr::new(
            result_type.clone(),
            resolved::Expr {
                kind: resolved::ExprKind::FloatExtend(Box::new(expr.clone()), result_type),
                source: expr.source,
            },
        ));
    }

    None
}

fn conform_integer_value(
    expr: &resolved::Expr,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    type_source: Source,
) -> Option<TypedExpr> {
    if from_sign != to_sign && !(to_bits > from_bits) {
        return None;
    }

    if to_bits < from_bits {
        return None;
    }

    let result_type = resolved::TypeKind::Integer {
        bits: to_bits,
        sign: to_sign,
    }
    .at(type_source);

    if from_sign == to_sign && to_bits == from_bits {
        return Some(TypedExpr::new(result_type, expr.clone()));
    }

    Some(TypedExpr::new(
        result_type.clone(),
        resolved::Expr {
            kind: resolved::ExprKind::IntegerExtend(Box::new(expr.clone()), result_type),
            source: expr.source,
        },
    ))
}

fn conform_expr_to_default(expr: TypedExpr) -> Result<TypedExpr, ResolveError> {
    match &expr.resolved_type.kind {
        resolved::TypeKind::IntegerLiteral(value) => {
            conform_integer_to_default_or_error(&value, expr.expr.source)
        }
        resolved::TypeKind::FloatLiteral(value) => {
            Ok(conform_float_to_default(*value, expr.expr.source))
        }
        _ => Ok(expr),
    }
}

fn conform_float_to_default(value: f64, source: Source) -> TypedExpr {
    TypedExpr::new(
        resolved::TypeKind::Float(FloatSize::Normal).at(source),
        resolved::Expr::new(resolved::ExprKind::Float(FloatSize::Normal, value), source),
    )
}

fn conform_integer_to_default_or_error(
    value: &BigInt,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    match conform_integer_to_default(&value, source) {
        Some(resolved) => Ok(resolved),
        None => Err(ResolveErrorKind::UnrepresentableInteger {
            value: value.to_string(),
        }
        .at(source)),
    }
}

fn conform_integer_to_default(value: &BigInt, source: Source) -> Option<TypedExpr> {
    use resolved::{IntegerBits::*, IntegerSign::*};

    let possible_types = [
        resolved::TypeKind::Integer {
            bits: Bits64,
            sign: Signed,
        }
        .at(source),
        resolved::TypeKind::Integer {
            bits: Bits64,
            sign: Unsigned,
        }
        .at(source),
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
) -> Option<TypedExpr> {
    match &to_type.kind {
        resolved::TypeKind::Float(size) => value.to_f64().map(|literal| {
            TypedExpr::new(
                resolved::TypeKind::Float(*size).at(source),
                resolved::Expr::new(resolved::ExprKind::Float(*size, literal), source),
            )
        }),
        resolved::TypeKind::Integer { bits, sign } => {
            use resolved::{IntegerBits::*, IntegerLiteralBits, IntegerSign::*};

            let make_integer = |integer_literal_bits| {
                Some(TypedExpr::new(
                    resolved::TypeKind::Integer {
                        bits: *bits,
                        sign: *sign,
                    }
                    .at(source),
                    resolved::Expr::new(
                        resolved::ExprKind::Integer {
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
        ast::TypeKind::Boolean => Ok(resolved::TypeKind::Boolean),
        ast::TypeKind::Integer { bits, sign } => Ok(resolved::TypeKind::Integer {
            bits: *bits,
            sign: *sign,
        }),
        ast::TypeKind::Pointer(inner) => {
            let inner = match resolve_type(type_search_context, source_file_cache, &inner) {
                Ok(inner) => inner,
                Err(_) if inner.kind.allow_undeclared() => {
                    resolved::TypeKind::Void.at(inner.source)
                }
                Err(err) => return Err(err),
            };

            Ok(resolved::TypeKind::Pointer(Box::new(inner)))
        }
        ast::TypeKind::Void => Ok(resolved::TypeKind::Void),
        ast::TypeKind::Named(name) => type_search_context
            .find_type_or_error(&name, ast_type.source)
            .cloned(),
        ast::TypeKind::PlainOldData(inner) => match &inner.kind {
            ast::TypeKind::Named(name) => {
                let resolved_inner_kind = type_search_context
                    .find_type_or_error(&name, ast_type.source)
                    .cloned()?;

                let structure_ref = match resolved_inner_kind {
                    resolved::TypeKind::ManagedStructure(_, structure_ref) => structure_ref,
                    resolved::TypeKind::PlainOldData(_, structure_ref) => structure_ref,
                    _ => {
                        return Err(ResolveErrorKind::CannotCreatePlainOldDataOfNonStructure {
                            bad_type: inner.to_string(),
                        }
                        .at(inner.source));
                    }
                };

                Ok(resolved::TypeKind::PlainOldData(
                    name.clone(),
                    structure_ref,
                ))
            }
            _ => Err(ResolveErrorKind::CannotCreatePlainOldDataOfNonStructure {
                bad_type: inner.to_string(),
            }
            .at(inner.source)),
        },
        ast::TypeKind::Float(size) => Ok(resolved::TypeKind::Float(*size)),
        ast::TypeKind::AnonymousStruct(..) => todo!("resolve anonymous struct type"),
        ast::TypeKind::AnonymousUnion(..) => todo!("resolve anonymous union type"),
        ast::TypeKind::AnonymousEnum(..) => todo!("resolve anonymous enum type"),
        ast::TypeKind::FixedArray(fixed_array) => {
            if let ast::ExprKind::Integer(integer) = &fixed_array.count.kind {
                if let Ok(size) = integer.value().try_into() {
                    let inner = resolve_type(
                        type_search_context,
                        source_file_cache,
                        &fixed_array.ast_type,
                    )?;

                    Ok(resolved::TypeKind::FixedArray(Box::new(
                        resolved::FixedArray { size, inner },
                    )))
                } else {
                    Err(ResolveErrorKind::ArraySizeTooLarge.at(fixed_array.count.source))
                }
            } else {
                todo!("resolve fixed array type with variable size")
            }
        }
        ast::TypeKind::FunctionPointer(function_pointer) => {
            let mut parameters = Vec::with_capacity(function_pointer.parameters.len());

            for parameter in function_pointer.parameters.iter() {
                let resolved_type =
                    resolve_type(type_search_context, source_file_cache, &parameter.ast_type)?;

                parameters.push(resolved::Parameter {
                    name: parameter.name.clone(),
                    resolved_type,
                });
            }

            let return_type = Box::new(resolve_type(
                type_search_context,
                source_file_cache,
                &function_pointer.return_type,
            )?);

            Ok(resolved::TypeKind::FunctionPointer(
                resolved::FunctionPointer {
                    parameters,
                    return_type,
                    is_cstyle_variadic: function_pointer.is_cstyle_variadic,
                },
            ))
        }
    }
    .map(|kind| kind.at(ast_type.source))
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
                    variable_name: variable_name.clone(),
                }
            }
            _ => ResolveErrorKind::CannotUseUninitializedValue,
        }
        .at(subject.source))
    }
}
