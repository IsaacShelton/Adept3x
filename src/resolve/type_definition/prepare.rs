use crate::{
    ast::{self, AstWorkspace, Privacy},
    name::{Name, ResolvedName},
    resolve::{
        ctx::ResolveCtx,
        error::{ResolveError, ResolveErrorKind},
        job::TypeJob,
        type_ctx::{resolve_constraints, ResolveTypeCtx},
    },
    resolved::{
        self, CurrentConstraints, EnumRef, HumanName, StructureRef, TraitRef, TypeAliasRef,
        TypeDecl, TypeParameters,
    },
    source_files::Source,
    workspace::fs::FsNodeId,
};
use indexmap::IndexMap;
use itertools::Itertools;

pub fn prepare_type_jobs(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
) -> Result<Vec<TypeJob>, ResolveError> {
    let mut type_jobs = Vec::with_capacity(ast_workspace.files.len());

    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_fs_node_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        let mut job = TypeJob {
            physical_file_id: *physical_file_id,
            type_aliases: Vec::with_capacity(file.type_aliases.len()),
            traits: Vec::with_capacity(file.traits.len()),
            structures: Vec::with_capacity(file.structures.len()),
            enums: Vec::with_capacity(file.enums.len()),
        };

        for user_trait in file.traits.iter() {
            job.traits.push(prepare_trait(
                ctx,
                resolved_ast,
                module_fs_node_id,
                user_trait,
            )?);
        }

        for structure in file.structures.iter() {
            job.structures.push(prepare_structure(
                ctx,
                resolved_ast,
                module_fs_node_id,
                *physical_file_id,
                structure,
            )?);
        }

        for definition in file.enums.iter() {
            job.enums.push(prepare_enum(
                ctx,
                resolved_ast,
                module_fs_node_id,
                definition,
            )?);
        }

        for definition in file.type_aliases.iter() {
            job.type_aliases.push(prepare_type_alias(
                ctx,
                resolved_ast,
                module_fs_node_id,
                definition,
            )?);
        }

        type_jobs.push(job);
    }

    Ok(type_jobs)
}

fn prepare_structure(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_fs_node_id: FsNodeId,
    physical_fs_node_id: FsNodeId,
    structure: &ast::Structure,
) -> Result<StructureRef, ResolveError> {
    let mut parameters = TypeParameters::default();

    for (name, parameter) in structure.parameters.iter() {
        let zero_current_constraints = CurrentConstraints::new_empty(ctx.implementations);
        let constraints = resolve_constraints(
            &ResolveTypeCtx::new(
                resolved_ast,
                module_fs_node_id,
                physical_fs_node_id,
                &ctx.types_in_modules,
                &zero_current_constraints,
            ),
            &parameter.constraints,
        )?;

        if parameters
            .parameters
            .insert(name.to_string(), resolved::TypeParameter { constraints })
            .is_some()
        {
            todo!("Error message for duplicate type parameter names")
        }
    }

    let structure_ref = resolved_ast.structures.insert(resolved::Structure {
        name: ResolvedName::new(module_fs_node_id, &Name::plain(&structure.name)),
        fields: IndexMap::new(),
        is_packed: structure.is_packed,
        parameters,
        source: structure.source,
    });

    // TODO: Improve the source tracking for these
    let polymorphs = structure
        .parameters
        .keys()
        .map(|name| resolved::TypeKind::Polymorph(name.into(), vec![]).at(structure.source))
        .collect_vec();

    declare_type(
        ctx,
        module_fs_node_id,
        &structure.name,
        structure.source,
        structure.privacy,
        resolved::TypeKind::Structure(
            HumanName(structure.name.to_string()),
            structure_ref,
            polymorphs,
        ),
    )?;

    Ok(structure_ref)
}

fn prepare_enum(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_fs_node_id: FsNodeId,
    definition: &ast::Enum,
) -> Result<EnumRef, ResolveError> {
    let enum_ref = resolved_ast.enums.insert(resolved::Enum {
        name: ResolvedName::new(module_fs_node_id, &Name::plain(&definition.name)),
        resolved_type: resolved::TypeKind::Unresolved.at(definition.source),
        source: definition.source,
        members: definition.members.clone(),
    });

    declare_type(
        ctx,
        module_fs_node_id,
        &definition.name,
        definition.source,
        definition.privacy,
        resolved::TypeKind::Enum(HumanName(definition.name.to_string()), enum_ref),
    )?;

    Ok(enum_ref)
}

fn prepare_trait(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_fs_node_id: FsNodeId,
    definition: &ast::Trait,
) -> Result<TraitRef, ResolveError> {
    let trait_ref = resolved_ast.traits.insert(resolved::Trait {
        methods: vec![],
        source: definition.source,
    });

    declare_type(
        ctx,
        module_fs_node_id,
        &definition.name,
        definition.source,
        definition.privacy,
        resolved::TypeKind::Trait(HumanName(definition.name.to_string()), trait_ref),
    )?;

    Ok(trait_ref)
}

fn prepare_type_alias(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_fs_node_id: FsNodeId,
    definition: &ast::TypeAlias,
) -> Result<TypeAliasRef, ResolveError> {
    let type_alias_ref = resolved_ast
        .type_aliases
        .insert(resolved::TypeKind::Unresolved.at(definition.value.source));

    if let Some(source) = definition.value.contains_polymorph() {
        return Err(ResolveErrorKind::Other {
            message: "Type aliases cannot contain polymorphs".into(),
        }
        .at(source));
    }

    declare_type(
        ctx,
        module_fs_node_id,
        &definition.name,
        definition.source,
        definition.privacy,
        resolved::TypeKind::TypeAlias(HumanName(definition.name.to_string()), type_alias_ref),
    )?;

    Ok(type_alias_ref)
}

fn declare_type(
    ctx: &mut ResolveCtx,
    module_fs_node_id: FsNodeId,
    name: &str,
    source: Source,
    privacy: Privacy,
    kind: resolved::TypeKind,
) -> Result<(), ResolveError> {
    if ctx
        .types_in_modules
        .entry(module_fs_node_id)
        .or_default()
        .insert(
            name.to_string(),
            TypeDecl {
                kind,
                source,
                privacy,
            },
        )
        .is_some()
    {
        return Err(ResolveErrorKind::DuplicateTypeName {
            name: name.to_string(),
        }
        .at(source));
    };

    Ok(())
}
