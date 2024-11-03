use crate::{
    ast::{self, AstWorkspace},
    name::{Name, ResolvedName},
    resolve::{
        ctx::ResolveCtx,
        error::{ResolveError, ResolveErrorKind},
        job::TypeJob,
    },
    resolved::{self, EnumRef, HumanName, StructureRef, TypeAliasRef, TypeDecl},
    workspace::fs::FsNodeId,
};
use indexmap::IndexMap;

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
            structures: Vec::with_capacity(file.structures.len()),
            enums: Vec::with_capacity(file.enums.len()),
        };

        for structure in file.structures.iter() {
            job.structures.push(prepare_structure(
                ctx,
                resolved_ast,
                module_fs_node_id,
                structure,
            ));
        }

        for definition in file.enums.iter() {
            job.enums.push(prepare_enum(
                ctx,
                resolved_ast,
                module_fs_node_id,
                definition,
            ));
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
    structure: &ast::Structure,
) -> StructureRef {
    let source = structure.source;

    let structure_ref = resolved_ast.structures.insert(resolved::Structure {
        name: ResolvedName::new(module_fs_node_id, &Name::plain(&structure.name)),
        fields: IndexMap::new(),
        is_packed: structure.is_packed,
        source: structure.source,
    });

    let struct_type_kind =
        resolved::TypeKind::Structure(HumanName(structure.name.to_string()), structure_ref);

    ctx.types_in_modules
        .entry(module_fs_node_id)
        .or_default()
        .insert(
            structure.name.to_string(),
            TypeDecl {
                kind: struct_type_kind,
                source,
                privacy: structure.privacy,
            },
        );

    structure_ref
}

fn prepare_enum(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_fs_node_id: FsNodeId,
    definition: &ast::Enum,
) -> EnumRef {
    let enum_ref = resolved_ast.enums.insert(resolved::Enum {
        name: ResolvedName::new(module_fs_node_id, &Name::plain(&definition.name)),
        resolved_type: resolved::TypeKind::Unresolved.at(definition.source),
        source: definition.source,
        members: definition.members.clone(),
    });

    let kind = resolved::TypeKind::Enum(HumanName(definition.name.to_string()), enum_ref);
    let source = definition.source;
    let privacy = definition.privacy;

    ctx.types_in_modules
        .entry(module_fs_node_id)
        .or_default()
        .insert(
            definition.name.to_string(),
            TypeDecl {
                kind,
                source,
                privacy,
            },
        );

    enum_ref
}

fn prepare_type_alias(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_fs_node_id: FsNodeId,
    definition: &ast::TypeAlias,
) -> Result<TypeAliasRef, ResolveError> {
    let source = definition.source;
    let type_alias_ref = resolved_ast
        .type_aliases
        .insert(resolved::TypeKind::Unresolved.at(definition.value.source));

    if let Some(source) = definition.value.contains_polymorph() {
        return Err(ResolveErrorKind::Other {
            message: "Type aliases cannot contain polymorphs".into(),
        }
        .at(source));
    }

    ctx.types_in_modules
        .entry(module_fs_node_id)
        .or_default()
        .insert(
            definition.name.to_string(),
            TypeDecl {
                kind: resolved::TypeKind::TypeAlias(
                    HumanName(definition.name.to_string()),
                    type_alias_ref,
                ),
                source,
                privacy: definition.privacy,
            },
        );

    Ok(type_alias_ref)
}
