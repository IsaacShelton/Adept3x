use crate::{
    asg::{
        self, Asg, CurrentConstraints, EnumRef, HumanName, StructRef, TraitRef, TypeAliasRef,
        TypeDecl, TypeParameters,
    },
    ast::{self, AstWorkspace, Privacy},
    name::{Name, ResolvedName},
    resolve::{
        ctx::ResolveCtx,
        error::{ResolveError, ResolveErrorKind},
        job::TypeJob,
        type_ctx::{resolve_constraints, ResolveTypeCtx},
    },
    source_files::Source,
    workspace::fs::FsNodeId,
};
use indexmap::IndexMap;
use itertools::Itertools;

pub fn prepare_type_jobs(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
) -> Result<Vec<TypeJob>, ResolveError> {
    let mut type_jobs = Vec::with_capacity(ast_workspace.files.len());

    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_fs_node_id = ast_workspace.get_owning_module_or_self(*physical_file_id);

        let mut job = TypeJob {
            physical_file_id: *physical_file_id,
            type_aliases: Vec::with_capacity(file.type_aliases.len()),
            traits: Vec::with_capacity(file.traits.len()),
            structs: Vec::with_capacity(file.structs.len()),
            enums: Vec::with_capacity(file.enums.len()),
        };

        for user_trait in file.traits.iter() {
            job.traits
                .push(prepare_trait(ctx, asg, module_fs_node_id, user_trait)?);
        }

        for structure in file.structs.iter() {
            job.structs.push(prepare_structure(
                ctx,
                asg,
                module_fs_node_id,
                *physical_file_id,
                structure,
            )?);
        }

        for definition in file.enums.iter() {
            job.enums
                .push(prepare_enum(ctx, asg, module_fs_node_id, definition)?);
        }

        for definition in file.type_aliases.iter() {
            job.type_aliases
                .push(prepare_type_alias(ctx, asg, module_fs_node_id, definition)?);
        }

        type_jobs.push(job);
    }

    Ok(type_jobs)
}

fn prepare_structure(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_fs_node_id: FsNodeId,
    physical_fs_node_id: FsNodeId,
    structure: &ast::Struct,
) -> Result<StructRef, ResolveError> {
    let mut parameters = TypeParameters::default();

    for (name, parameter) in structure.params.iter() {
        let zero_current_constraints = CurrentConstraints::new_empty();
        let constraints = resolve_constraints(
            &ResolveTypeCtx::new(
                asg,
                module_fs_node_id,
                physical_fs_node_id,
                &ctx.types_in_modules,
                &zero_current_constraints,
            ),
            &parameter.constraints,
        )?;

        if parameters
            .parameters
            .insert(name.to_string(), asg::TypeParameter { constraints })
            .is_some()
        {
            todo!("Error message for duplicate type parameter names")
        }
    }

    let struct_ref = asg.structs.insert(asg::Struct {
        name: ResolvedName::new(module_fs_node_id, &Name::plain(&structure.name)),
        fields: IndexMap::new(),
        is_packed: structure.is_packed,
        parameters,
        source: structure.source,
    });

    // TODO: Improve the source tracking for these
    let polymorphs = structure
        .params
        .keys()
        .map(|name| asg::TypeKind::Polymorph(name.into(), vec![]).at(structure.source))
        .collect_vec();

    declare_type(
        ctx,
        module_fs_node_id,
        &structure.name,
        structure.source,
        structure.privacy,
        asg::TypeKind::Structure(
            HumanName(structure.name.to_string()),
            struct_ref,
            polymorphs,
        ),
    )?;

    Ok(struct_ref)
}

fn prepare_enum(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_fs_node_id: FsNodeId,
    definition: &ast::Enum,
) -> Result<EnumRef, ResolveError> {
    let enum_ref = asg.enums.insert(asg::Enum {
        name: ResolvedName::new(module_fs_node_id, &Name::plain(&definition.name)),
        ty: asg::TypeKind::Unresolved.at(definition.source),
        source: definition.source,
        members: definition.members.clone(),
    });

    declare_type(
        ctx,
        module_fs_node_id,
        &definition.name,
        definition.source,
        definition.privacy,
        asg::TypeKind::Enum(HumanName(definition.name.to_string()), enum_ref),
    )?;

    Ok(enum_ref)
}

fn prepare_trait(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_fs_node_id: FsNodeId,
    definition: &ast::Trait,
) -> Result<TraitRef, ResolveError> {
    let trait_ref = asg.traits.insert(asg::Trait {
        functions: vec![],
        parameters: definition.parameters.clone(),
        source: definition.source,
    });

    let parameters = definition
        .parameters
        .iter()
        .map(|name| asg::TypeKind::Polymorph(name.clone(), vec![]).at(definition.source))
        .collect_vec();

    declare_type(
        ctx,
        module_fs_node_id,
        &definition.name,
        definition.source,
        definition.privacy,
        asg::TypeKind::Trait(
            HumanName(definition.name.to_string()),
            trait_ref,
            parameters,
        ),
    )?;

    Ok(trait_ref)
}

fn prepare_type_alias(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_fs_node_id: FsNodeId,
    definition: &ast::TypeAlias,
) -> Result<TypeAliasRef, ResolveError> {
    let type_alias_ref = asg
        .type_aliases
        .insert(asg::TypeKind::Unresolved.at(definition.value.source));

    if let Some(source) = definition.value.contains_polymorph() {
        return Err(ResolveErrorKind::TypeAliasesCannotContainPolymorphs.at(source));
    }

    declare_type(
        ctx,
        module_fs_node_id,
        &definition.name,
        definition.source,
        definition.privacy,
        asg::TypeKind::TypeAlias(HumanName(definition.name.to_string()), type_alias_ref),
    )?;

    Ok(type_alias_ref)
}

fn declare_type(
    ctx: &mut ResolveCtx,
    module_fs_node_id: FsNodeId,
    name: &str,
    source: Source,
    privacy: Privacy,
    kind: asg::TypeKind,
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
