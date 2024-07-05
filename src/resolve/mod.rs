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
    ast::{self, Ast, ConformBehavior, FileIdentifier, Source, Type},
    resolved::{self, Enum, TypedExpr, VariableStorage},
    source_file_cache::SourceFileCache,
    try_insert_index_map::try_insert_into_index_map,
};
use ast::{FloatSize, IntegerBits, IntegerSign};
use function_search_ctx::FunctionSearchCtx;
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet, VecDeque},
};

enum Job {
    Regular(FileIdentifier, usize, resolved::FunctionRef),
}

struct ResolveCtx<'a> {
    pub jobs: VecDeque<Job>,
    pub type_search_ctxs: HashMap<String, TypeSearchCtx<'a>>,
    pub function_search_contexts: HashMap<String, FunctionSearchCtx<'a>>,
    pub global_search_contexts: HashMap<String, GlobalSearchCtx<'a>>,
    pub defines: IndexMap<String, &'a ast::Define>,
}

impl<'a> ResolveCtx<'a> {
    fn new(defines: IndexMap<String, &'a ast::Define>) -> Self {
        Self {
            jobs: Default::default(),
            type_search_ctxs: Default::default(),
            function_search_contexts: Default::default(),
            global_search_contexts: Default::default(),
            defines,
        }
    }
}

pub fn resolve<'a>(ast: &'a Ast) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut defines = IndexMap::new();

    // Unify defines into single map
    for (_, file) in ast.files.iter() {
        for (define_name, define) in file.defines.iter() {
            try_insert_into_index_map(&mut defines, define_name.clone(), define, |define_name| {
                ResolveErrorKind::MultipleDefinesNamed { name: define_name }.at(define.source)
            })?;
        }
    }

    let mut ctx = ResolveCtx::new(defines);
    let source_file_cache = ast.source_file_cache;
    let mut resolved_ast = resolved::Ast::new(source_file_cache);

    let mut aliases: IndexMap<String, &'a ast::Alias> = IndexMap::new();

    // Unify type aliases into single map
    for (_, file) in ast.files.iter() {
        for (alias_name, alias) in file.aliases.iter() {
            try_insert_into_index_map(&mut aliases, alias_name.clone(), alias, |alias_name| {
                ResolveErrorKind::MultipleDefinitionsOfTypeNamed { name: alias_name }
                    .at(alias.source)
            })?;
        }
    }

    let type_search_ctx = ctx
        .type_search_ctxs
        .entry(ast.primary_filename.clone())
        .or_insert_with(|| TypeSearchCtx::new(source_file_cache, aliases));

    // Temporarily used stack to keep track of used type aliases
    let mut used_aliases = HashSet::<String>::new();

    // Pre-compute resolved enum types
    for (_, file) in ast.files.iter() {
        for (enum_name, enum_definition) in file.enums.iter() {
            let resolved_type = resolve_enum_backing_type(
                type_search_ctx,
                source_file_cache,
                enum_definition.backing_type.as_ref(),
                &mut used_aliases,
                enum_definition.source,
            )?;

            let members = enum_definition.members.clone();

            resolved_ast.enums.insert(
                enum_name.clone(),
                Enum {
                    resolved_type,
                    source: enum_definition.source,
                    members,
                },
            );

            type_search_ctx.put(
                enum_name.clone(),
                resolved::TypeKind::Enum(enum_name.clone()),
                enum_definition.source,
            )?;
        }
    }

    // Precompute resolved struct types
    for (_, file) in ast.files.iter() {
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
                            &mut used_aliases,
                        )?,
                        privacy: field.privacy,
                        source: field.source,
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
                    structure.source,
                )?;
            } else {
                type_search_ctx.put(
                    structure.name.clone(),
                    resolved::TypeKind::ManagedStructure(structure.name.clone(), structure_key),
                    structure.source,
                )?;
            }
        }
    }

    // Resolve type aliases
    for (_, file) in ast.files.iter() {
        for (alias_name, alias) in file.aliases.iter() {
            let resolved_type = resolve_type_or_undeclared(
                type_search_ctx,
                source_file_cache,
                &alias.value,
                &mut used_aliases,
            )?;

            type_search_ctx.put(alias_name.clone(), resolved_type.kind, alias.source)?;
        }
    }

    let global_search_context = ctx
        .global_search_contexts
        .entry(ast.primary_filename.clone())
        .or_insert_with(|| GlobalSearchCtx::new(source_file_cache));

    // Resolve global variables
    for (_, file) in ast.files.iter() {
        for global in file.globals.iter() {
            let resolved_type = resolve_type(
                type_search_ctx,
                source_file_cache,
                &global.ast_type,
                &mut Default::default(),
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
    }

    // Create initial function jobs
    for (file_identifier, file) in ast.files.iter() {
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
                    &mut Default::default(),
                )?,
                stmts: vec![],
                is_foreign: function.is_foreign,
                variables: VariableStorage::new(),
                source: function.source,
                abide_abi: function.abide_abi,
                tag: function.tag,
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
                    .type_search_ctxs
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
                        let resolved_type = resolve_type(
                            type_search_ctx,
                            source_file_cache,
                            &parameter.ast_type,
                            &mut Default::default(),
                        )?;
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
                        defines: &ctx.defines,
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
    behavior: ConformBehavior,
    conform_source: Source,
) -> Result<TypedExpr, ResolveError> {
    if let Some(expr) = conform_expr(expr, target_type, mode, behavior, conform_source) {
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

    pub fn allow_lossy_integer(&self) -> bool {
        match self {
            Self::Explicit => true,
            _ => false,
        }
    }
}

fn conform_expr(
    expr: &TypedExpr,
    to_type: &resolved::Type,
    mode: ConformMode,
    conform_behavior: ConformBehavior,
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
                resolved::TypeKind::Integer { bits, sign } => match conform_behavior {
                    ConformBehavior::Adept => conform_integer_value(
                        &expr.expr,
                        *from_bits,
                        *from_sign,
                        *bits,
                        *sign,
                        to_type.source,
                    ),
                    ConformBehavior::C => conform_integer_value_c(
                        &expr.expr,
                        mode,
                        *from_bits,
                        *from_sign,
                        *bits,
                        *sign,
                        to_type.source,
                    ),
                },
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

fn conform_integer_value_c(
    expr: &resolved::Expr,
    conform_mode: ConformMode,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    type_source: Source,
) -> Option<TypedExpr> {
    let result_type = resolved::TypeKind::Integer {
        bits: to_bits,
        sign: to_sign,
    }
    .at(type_source);

    if from_bits == to_bits && from_sign == to_sign {
        return Some(TypedExpr::new(result_type, expr.clone()));
    }

    if conform_mode.allow_lossy_integer() {
        let kind = if from_bits < to_bits {
            resolved::ExprKind::IntegerExtend(Box::new(expr.clone()), result_type.clone())
        } else {
            resolved::ExprKind::IntegerTruncate(Box::new(expr.clone()), result_type.clone())
        };

        return Some(TypedExpr::new(
            result_type,
            resolved::Expr {
                kind,
                source: expr.source,
            },
        ));
    }

    todo!("conform_integer_value_c {:?}", conform_mode);
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

#[derive(Copy, Clone, Debug)]
enum Initialized {
    Require,
    AllowUninitialized,
}

fn resolve_type_or_undeclared<'a>(
    type_search_ctx: &'a TypeSearchCtx<'_>,
    source_file_cache: &SourceFileCache,
    ast_type: &'a ast::Type,
    used_aliases_stack: &mut HashSet<String>,
) -> Result<resolved::Type, ResolveError> {
    match resolve_type(
        type_search_ctx,
        source_file_cache,
        ast_type,
        used_aliases_stack,
    ) {
        Ok(inner) => Ok(inner),
        Err(_) if ast_type.kind.allow_undeclared() => {
            Ok(resolved::TypeKind::Void.at(ast_type.source))
        }
        Err(err) => Err(err),
    }
}

fn resolve_type<'a>(
    type_search_ctx: &'a TypeSearchCtx<'_>,
    source_file_cache: &SourceFileCache,
    ast_type: &'a ast::Type,
    used_aliases_stack: &mut HashSet<String>,
) -> Result<resolved::Type, ResolveError> {
    match &ast_type.kind {
        ast::TypeKind::Boolean => Ok(resolved::TypeKind::Boolean),
        ast::TypeKind::Integer { bits, sign } => Ok(resolved::TypeKind::Integer {
            bits: *bits,
            sign: *sign,
        }),
        ast::TypeKind::CInteger { integer, sign } => Ok(resolved::TypeKind::CInteger {
            integer: *integer,
            sign: *sign,
        }),
        ast::TypeKind::Pointer(inner) => {
            let inner = resolve_type_or_undeclared(
                type_search_ctx,
                source_file_cache,
                &inner,
                used_aliases_stack,
            )?;

            Ok(resolved::TypeKind::Pointer(Box::new(inner)))
        }
        ast::TypeKind::Void => Ok(resolved::TypeKind::Void),
        ast::TypeKind::Named(name) => {
            let search = type_search_ctx
                .find_type_or_error(&name, ast_type.source)
                .cloned();

            match search {
                Ok(found) => Ok(found),
                Err(err) => {
                    if let Some(definition) = type_search_ctx.find_alias(name) {
                        if used_aliases_stack.insert(name.clone()) {
                            let inner = resolve_type(
                                type_search_ctx,
                                source_file_cache,
                                &definition.value,
                                used_aliases_stack,
                            );
                            used_aliases_stack.remove(name.as_str());
                            inner.map(|ty| ty.kind)
                        } else {
                            Err(ResolveErrorKind::RecursiveTypeAlias { name: name.clone() }
                                .at(definition.source))
                        }
                    } else {
                        Err(err)
                    }
                }
            }
        }
        ast::TypeKind::PlainOldData(inner) => match &inner.kind {
            ast::TypeKind::Named(name) => {
                let resolved_inner_kind = type_search_ctx
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
        ast::TypeKind::AnonymousEnum(anonymous_enum) => {
            let resolved_type = Box::new(resolve_enum_backing_type(
                type_search_ctx,
                source_file_cache,
                anonymous_enum.backing_type.as_deref(),
                &mut Default::default(),
                ast_type.source,
            )?);

            let members = anonymous_enum.members.clone();

            Ok(resolved::TypeKind::AnonymousEnum(resolved::AnonymousEnum {
                resolved_type,
                source: ast_type.source,
                members,
            }))
        }
        ast::TypeKind::FixedArray(fixed_array) => {
            if let ast::ExprKind::Integer(integer) = &fixed_array.count.kind {
                if let Ok(size) = integer.value().try_into() {
                    let inner = resolve_type(
                        type_search_ctx,
                        source_file_cache,
                        &fixed_array.ast_type,
                        used_aliases_stack,
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
                let resolved_type = resolve_type(
                    type_search_ctx,
                    source_file_cache,
                    &parameter.ast_type,
                    used_aliases_stack,
                )?;

                parameters.push(resolved::Parameter {
                    name: parameter.name.clone(),
                    resolved_type,
                });
            }

            let return_type = Box::new(resolve_type(
                type_search_ctx,
                source_file_cache,
                &function_pointer.return_type,
                used_aliases_stack,
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
    type_search_ctx: &TypeSearchCtx<'_>,
    source_file_cache: &SourceFileCache,
    parameters: &ast::Parameters,
) -> Result<resolved::Parameters, ResolveError> {
    let mut required = Vec::with_capacity(parameters.required.len());

    for parameter in parameters.required.iter() {
        required.push(resolved::Parameter {
            name: parameter.name.clone(),
            resolved_type: resolve_type(
                type_search_ctx,
                source_file_cache,
                &parameter.ast_type,
                &mut Default::default(),
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

fn resolve_enum_backing_type(
    type_search_ctx: &TypeSearchCtx,
    source_file_cache: &SourceFileCache,
    backing_type: Option<impl Borrow<Type>>,
    used_aliases: &mut HashSet<String>,
    source: Source,
) -> Result<resolved::Type, ResolveError> {
    if let Some(backing_type) = backing_type.as_ref().map(Borrow::borrow) {
        resolve_type(
            type_search_ctx,
            source_file_cache,
            backing_type,
            used_aliases,
        )
    } else {
        Ok(resolved::TypeKind::Integer {
            bits: IntegerBits::Bits64,
            sign: IntegerSign::Unsigned,
        }
        .at(source))
    }
}
