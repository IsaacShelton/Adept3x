mod conform;
mod core_structure_info;
mod destination;
mod error;
mod expr;
mod function_search_ctx;
mod global_search_ctx;
mod stmt;
mod unify_types;
mod variable_search_ctx;

use self::{
    error::{ResolveError, ResolveErrorKind},
    expr::ResolveExprCtx,
    global_search_ctx::GlobalSearchCtx,
    stmt::resolve_stmts,
    variable_search_ctx::VariableSearchCtx,
};
use crate::{
    ast::{self, AstWorkspace, Type},
    cli::BuildOptions,
    index_map_ext::IndexMapExt,
    name::{Name, ResolvedName},
    resolved::{
        self, HumanName, StructureRef, TypeDecl, TypeKind, TypeRef, TypedExpr, VariableStorage,
    },
    source_files::Source,
    tag::Tag,
    workspace::fs::FsNodeId,
};
use ast::{IntegerBits, IntegerSign};
use function_search_ctx::FunctionSearchCtx;
use indexmap::IndexMap;
use std::{
    borrow::{Borrow, Cow},
    collections::{HashMap, HashSet, VecDeque},
};

enum Job {
    Regular(FsNodeId, usize, resolved::FunctionRef),
}

struct ResolveCtx<'a> {
    pub jobs: VecDeque<Job>,
    pub function_search_ctxs: IndexMap<FsNodeId, FunctionSearchCtx>,
    pub global_search_ctxs: IndexMap<FsNodeId, GlobalSearchCtx>,
    pub helper_exprs: IndexMap<ResolvedName, &'a ast::HelperExpr>,
    pub public_functions: HashMap<FsNodeId, HashMap<String, Vec<resolved::FunctionRef>>>,
    pub types_in_modules: HashMap<FsNodeId, HashMap<String, resolved::TypeDecl>>,
}

impl<'a> ResolveCtx<'a> {
    fn new(helper_exprs: IndexMap<ResolvedName, &'a ast::HelperExpr>) -> Self {
        Self {
            jobs: Default::default(),
            function_search_ctxs: Default::default(),
            global_search_ctxs: Default::default(),
            helper_exprs,
            public_functions: HashMap::new(),
            types_in_modules: HashMap::new(),
        }
    }
}

pub fn resolve<'a>(
    ast_workspace: &'a AstWorkspace,
    options: &BuildOptions,
) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut helper_exprs = IndexMap::new();

    // Unify helper expressions into single map
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let file_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        if let Some(settings) = file.settings.map(|id| &ast_workspace.settings[id.0]) {
            if settings.debug_skip_merging_helper_exprs {
                continue;
            }
        }

        for (name, helper_expr) in file.helper_exprs.iter() {
            if !helper_expr.is_file_local_only {
                helper_exprs.try_insert(
                    ResolvedName::new(file_id, name),
                    helper_expr,
                    |define_name| {
                        ResolveErrorKind::MultipleDefinesNamed {
                            name: define_name.display(&ast_workspace.fs).to_string(),
                        }
                        .at(helper_expr.source)
                    },
                )?;
            }
        }
    }

    let mut ctx = ResolveCtx::new(helper_exprs);
    let source_files = ast_workspace.source_files;
    let mut resolved_ast = resolved::Ast::new(source_files, &ast_workspace);

    #[derive(Clone, Debug)]
    struct TypeJob<'a> {
        physical_file_id: FsNodeId,
        type_aliases: HashMap<&'a Name, TypeRef>,
        structures: Vec<StructureRef>,
        enums: HashMap<&'a Name, TypeRef>,
    }

    let mut type_jobs = Vec::with_capacity(ast_workspace.files.len());

    // Pre-resolve types for new type resolution system
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let file_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        let mut job = TypeJob {
            physical_file_id: *physical_file_id,
            type_aliases: HashMap::with_capacity(file.type_aliases.len()),
            structures: Vec::with_capacity(file.structures.len()),
            enums: HashMap::with_capacity(file.enums.len()),
        };

        for structure in file.structures.iter() {
            let privacy = structure.privacy;
            let source = structure.source;
            let resolved_name = ResolvedName::new(file_id, &structure.name);

            let structure_ref = resolved_ast.structures.insert(resolved::Structure {
                name: resolved_name.clone(),
                fields: IndexMap::new(),
                is_packed: structure.is_packed,
                source: structure.source,
            });

            let struct_type_kind =
                TypeKind::Structure(HumanName(structure.name.to_string()), structure_ref);

            let Some(name) = structure.name.as_plain_str() else {
                eprintln!(
                    "warning: internal namespaced structures ignored by new type resolution system"
                );
                continue;
            };

            let types_in_module = ctx
                .types_in_modules
                .entry(file_id)
                .or_insert_with(HashMap::new);

            types_in_module.insert(
                name.to_string(),
                TypeDecl {
                    kind: struct_type_kind,
                    source,
                    privacy,
                },
            );

            job.structures.push(structure_ref);
        }

        for (name, definition) in file.enums.iter() {
            let backing_type = definition
                .backing_type
                .is_some()
                .then(|| resolved_ast.all_types.insert(TypeKind::Unresolved));

            let kind = TypeKind::Enum(HumanName(name.to_string()), backing_type);
            let source = definition.source;
            let privacy = definition.privacy;

            let types_in_module = ctx
                .types_in_modules
                .entry(file_id)
                .or_insert_with(HashMap::new);

            types_in_module.insert(
                name.to_string(),
                TypeDecl {
                    kind,
                    source,
                    privacy,
                },
            );

            if let Some(backing_type) = backing_type {
                job.enums.insert(name, backing_type);
            }
        }

        for (name, definition) in file.type_aliases.iter() {
            let source = definition.source;
            let privacy = definition.privacy;
            let becomes_type = resolved_ast.all_types.insert(TypeKind::Unresolved);
            let kind = TypeKind::TypeAlias(HumanName(name.to_string()), becomes_type);

            let types_in_module = ctx
                .types_in_modules
                .entry(file_id)
                .or_insert_with(HashMap::new);

            types_in_module.insert(
                name.to_string(),
                TypeDecl {
                    kind,
                    source,
                    privacy,
                },
            );

            job.type_aliases.insert(name, becomes_type);
        }

        type_jobs.push(job);
    }

    // Create edges between types
    #[allow(dead_code, unused_variables)]
    for job in type_jobs.iter() {
        let file = ast_workspace
            .files
            .get(&job.physical_file_id)
            .expect("valid ast file");

        let module_file_id = ast_workspace
            .get_owning_module(job.physical_file_id)
            .unwrap_or(job.physical_file_id);

        let types = resolved_ast
            .types_per_module
            .entry(module_file_id)
            .or_default();

        for (structure_ref, structure) in job.structures.iter().zip(file.structures.iter()) {
            for (field_name, field) in structure.fields.iter() {
                let type_ctx =
                    ResolveTypeCtx::new(&resolved_ast, module_file_id, &ctx.types_in_modules);

                let resolved_type = type_ctx.resolve_or_undeclared(&field.ast_type)?;

                let resolved_struct = resolved_ast
                    .structures
                    .get_mut(*structure_ref)
                    .expect("valid struct");

                resolved_struct.fields.insert(
                    field_name.clone(),
                    resolved::Field {
                        resolved_type,
                        privacy: field.privacy,
                        source: field.source,
                    },
                );
            }

            eprintln!("warning - new type resolution does not handle struct fields yet");
        }

        for (name, type_ref) in job.enums.iter() {
            eprintln!("warning - new type resolution does not handle enum backing types yet");
        }

        for (name, type_ref) in job.type_aliases.iter() {
            eprintln!("warning - new type resolution does not handle enum type aliases yet");
        }
    }

    // Resolve global variables
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        for global in file.global_variables.iter() {
            let type_ctx =
                ResolveTypeCtx::new(&resolved_ast, module_file_id, &ctx.types_in_modules);
            let resolved_type = type_ctx.resolve(&global.ast_type)?;

            let global_search_context = ctx
                .global_search_ctxs
                .get_or_insert_with(module_file_id, || GlobalSearchCtx::new());

            let resolved_name = ResolvedName::new(module_file_id, &global.name);

            let global_ref = resolved_ast.globals.insert(resolved::GlobalVar {
                name: resolved_name.clone(),
                resolved_type: resolved_type.clone(),
                source: global.source,
                is_foreign: global.is_foreign,
                is_thread_local: global.is_thread_local,
            });

            global_search_context.put(resolved_name, resolved_type, global_ref);
        }
    }

    // Create initial function jobs
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        for (function_i, function) in file.functions.iter().enumerate() {
            let name = ResolvedName::new(module_file_id, &function.name);
            let type_ctx =
                ResolveTypeCtx::new(&resolved_ast, module_file_id, &ctx.types_in_modules);
            let parameters = resolve_parameters(&type_ctx, &function.parameters)?;
            let return_type = type_ctx.resolve(&function.return_type)?;

            let function_ref = resolved_ast.functions.insert(resolved::Function {
                name: name.clone(),
                parameters,
                return_type,
                stmts: vec![],
                is_foreign: function.is_foreign,
                variables: VariableStorage::new(),
                source: function.source,
                abide_abi: function.abide_abi,
                tag: function.tag.or_else(|| {
                    if options.coerce_main_signature && &*function.name.basename == "main" {
                        Some(Tag::Main)
                    } else {
                        None
                    }
                }),
            });

            ctx.jobs
                .push_back(Job::Regular(*physical_file_id, function_i, function_ref));

            if function.privacy.is_public() {
                let public_of_module = ctx
                    .public_functions
                    .entry(module_file_id)
                    .or_insert_with(HashMap::new);

                // TODO: Add proper error message
                let function_name = function
                    .name
                    .as_plain_str()
                    .expect("cannot make public symbol with existing namespace");

                if public_of_module.get(function_name).is_none() {
                    public_of_module.insert(function_name.to_string(), vec![]);
                }

                let functions_of_name = public_of_module
                    .get_mut(function_name)
                    .expect("function list inserted");
                functions_of_name.push(function_ref);
            }

            let settings = file.settings.map(|id| &ast_workspace.settings[id.0]);
            let imported_namespaces = settings.map(|settings| &settings.imported_namespaces);

            let function_search_context =
                ctx.function_search_ctxs
                    .get_or_insert_with(module_file_id, || {
                        FunctionSearchCtx::new(
                            imported_namespaces
                                .map(|namespaces| namespaces.clone())
                                .unwrap_or_else(|| vec![]),
                            module_file_id,
                        )
                    });

            function_search_context
                .available
                .entry(name)
                .and_modify(|funcs| funcs.push(function_ref))
                .or_insert_with(|| vec![function_ref]);
        }
    }

    // Resolve function bodies
    while let Some(job) = ctx.jobs.pop_front() {
        match job {
            Job::Regular(real_file_id, function_index, resolved_function_ref) => {
                let module_file_id = ast_workspace
                    .get_owning_module(real_file_id)
                    .unwrap_or(real_file_id);

                // NOTE: This module should already have a function search context
                let function_search_ctx = ctx
                    .function_search_ctxs
                    .get(&module_file_id)
                    .expect("function search context to exist for file");

                let global_search_ctx = &*ctx.global_search_ctxs.entry(module_file_id).or_default();

                let ast_file = ast_workspace
                    .files
                    .get(&real_file_id)
                    .expect("file referenced by job to exist");

                let ast_function = ast_file
                    .functions
                    .get(function_index)
                    .expect("function referenced by job to exist");

                let mut variable_search_ctx = VariableSearchCtx::new();

                {
                    for parameter in ast_function.parameters.required.iter() {
                        let type_ctx = ResolveTypeCtx::new(
                            &resolved_ast,
                            module_file_id,
                            &ctx.types_in_modules,
                        );

                        let resolved_type = type_ctx.resolve(&parameter.ast_type)?;

                        let function = resolved_ast
                            .functions
                            .get_mut(resolved_function_ref)
                            .unwrap();

                        let variable_key = function.variables.add_parameter(resolved_type.clone());

                        variable_search_ctx.put(
                            parameter.name.clone(),
                            resolved_type,
                            variable_key,
                        );
                    }
                }

                let file = ast_workspace
                    .files
                    .get(&real_file_id)
                    .expect("referenced file exists");

                let settings = &ast_workspace.settings[file.settings.unwrap_or_default().0];

                let resolved_stmts = {
                    let mut ctx = ResolveExprCtx {
                        resolved_ast: &mut resolved_ast,
                        function_search_ctx,
                        global_search_ctx,
                        variable_search_ctx,
                        resolved_function_ref,
                        helper_exprs: &ctx.helper_exprs,
                        settings,
                        public_functions: &ctx.public_functions,
                        types_in_modules: &ctx.types_in_modules,
                        module_fs_node_id: module_file_id,
                    };

                    resolve_stmts(&mut ctx, &ast_function.stmts)?
                };

                let resolved_function = resolved_ast
                    .functions
                    .get_mut(resolved_function_ref)
                    .expect("resolved function head to exist");

                resolved_function.stmts = resolved_stmts;
            }
        }
    }

    Ok(resolved_ast)
}

#[derive(Copy, Clone, Debug)]
enum Initialized {
    Require,
    AllowUninitialized,
}

#[derive(Debug)]
pub struct ResolveTypeCtx<'a> {
    resolved_ast: &'a resolved::Ast<'a>,
    module_fs_node_id: FsNodeId,
    types_in_modules: &'a HashMap<FsNodeId, HashMap<String, resolved::TypeDecl>>,
    used_aliases_stack: HashSet<ResolvedName>,
}

impl<'a> ResolveTypeCtx<'a> {
    pub fn new(
        resolved_ast: &'a resolved::Ast,
        module_fs_node_id: FsNodeId,
        public_types: &'a HashMap<FsNodeId, HashMap<String, resolved::TypeDecl>>,
    ) -> Self {
        Self {
            resolved_ast,
            module_fs_node_id,
            types_in_modules: public_types,
            used_aliases_stack: Default::default(),
        }
    }

    pub fn resolve_or_undeclared(
        &self,
        ast_type: &'a ast::Type,
    ) -> Result<resolved::Type, ResolveError> {
        match self.resolve(ast_type) {
            Ok(inner) => Ok(inner),
            Err(_) if ast_type.kind.allow_indirect_undefined() => {
                Ok(resolved::TypeKind::Void.at(ast_type.source))
            }
            Err(err) => Err(err),
        }
    }

    pub fn resolve(&self, ast_type: &'a ast::Type) -> Result<resolved::Type, ResolveError> {
        match &ast_type.kind {
            ast::TypeKind::Boolean => Ok(resolved::TypeKind::Boolean),
            ast::TypeKind::Integer(bits, sign) => Ok(resolved::TypeKind::Integer(*bits, *sign)),
            ast::TypeKind::CInteger(integer, sign) => {
                Ok(resolved::TypeKind::CInteger(*integer, *sign))
            }
            ast::TypeKind::Pointer(inner) => {
                let inner = self.resolve_or_undeclared(inner)?;
                Ok(resolved::TypeKind::Pointer(Box::new(inner)))
            }
            ast::TypeKind::Void => Ok(resolved::TypeKind::Void),
            ast::TypeKind::Named(name) => match self.find_type(name) {
                Ok(found) => Ok(found.into_owned()),
                Err(err) => Err(err.into_resolve_error(name, ast_type.source)),
            },
            ast::TypeKind::Floating(size) => Ok(resolved::TypeKind::Floating(*size)),
            ast::TypeKind::AnonymousStruct(..) => todo!("resolve anonymous struct type"),
            ast::TypeKind::AnonymousUnion(..) => todo!("resolve anonymous union type"),
            ast::TypeKind::AnonymousEnum(anonymous_enum) => {
                let resolved_type = Box::new(resolve_enum_backing_type(
                    self,
                    anonymous_enum.backing_type.as_deref(),
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
                        let inner = self.resolve(&fixed_array.ast_type)?;

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
                    let resolved_type = self.resolve(&parameter.ast_type)?;

                    parameters.push(resolved::Parameter {
                        name: parameter.name.clone(),
                        resolved_type,
                    });
                }

                let return_type = Box::new(self.resolve(&function_pointer.return_type)?);

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

    pub fn find_type(&self, name: &Name) -> Result<Cow<'a, resolved::TypeKind>, FindTypeError> {
        let _source_files = self.resolved_ast.source_files;
        let _settings = self
            .resolved_ast
            .workspace
            .get_settings_for_module(self.module_fs_node_id);
        let _all_types = &self.resolved_ast.all_types;

        if let Some(name) = name.as_plain_str() {
            if let Some(types_in_modules) = self.types_in_modules.get(&self.module_fs_node_id) {
                if let Some(decl) = types_in_modules.get(name) {
                    return Ok(Cow::Borrowed(&decl.kind));
                }
            }
        }

        todo!("TypeSearchCtx find_type - {:?}", name);

        /*
        let resolved_name = ResolvedName::new(self.fs_node_id, name);

        if let Some(mapping) = self.types.get(&resolved_name) {
            return self.resolve_mapping(&resolved_name, mapping, used_aliases_stack);
        }

        if name.namespace.is_empty() {
            let mut matches = self
                .settings
                .imported_namespaces
                .iter()
                .filter_map(|namespace| {
                    let resolved_name = ResolvedName::new(
                        self.fs_node_id,
                        &Name::new(Some(namespace.clone()), name.basename.clone()),
                    );
                    self.types.get(&resolved_name)
                });

            if let Some(found) = matches.next() {
                if matches.next().is_some() {
                    return Err(FindTypeError::Ambiguous);
                } else {
                    return self.resolve_mapping(&resolved_name, found, used_aliases_stack);
                }
            }
        }
        */

        Err(FindTypeError::NotDefined)
    }
}

#[derive(Clone, Debug)]
pub enum FindTypeError {
    NotDefined,
    Ambiguous,
    RecursiveAlias(ResolvedName),
    ResolveError(ResolveError),
}

impl FindTypeError {
    pub fn into_resolve_error(self: FindTypeError, name: &Name, source: Source) -> ResolveError {
        let name = name.to_string();

        match self {
            FindTypeError::NotDefined => ResolveErrorKind::UndeclaredType {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::Ambiguous => ResolveErrorKind::AmbiguousType {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::RecursiveAlias(_) => ResolveErrorKind::RecursiveTypeAlias {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::ResolveError(err) => err,
        }
    }
}

fn resolve_parameters(
    type_ctx: &ResolveTypeCtx,
    parameters: &ast::Parameters,
) -> Result<resolved::Parameters, ResolveError> {
    let mut required = Vec::with_capacity(parameters.required.len());

    for parameter in parameters.required.iter() {
        required.push(resolved::Parameter {
            name: parameter.name.clone(),
            resolved_type: type_ctx.resolve(&parameter.ast_type)?,
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
                    variable_name: variable_name.to_string(),
                }
            }
            _ => ResolveErrorKind::CannotUseUninitializedValue,
        }
        .at(subject.source))
    }
}

fn resolve_enum_backing_type(
    ctx: &ResolveTypeCtx,
    backing_type: Option<impl Borrow<Type>>,
    source: Source,
) -> Result<resolved::Type, ResolveError> {
    if let Some(backing_type) = backing_type.as_ref().map(Borrow::borrow) {
        ctx.resolve(backing_type)
    } else {
        Ok(resolved::TypeKind::Integer(IntegerBits::Bits64, IntegerSign::Unsigned).at(source))
    }
}
