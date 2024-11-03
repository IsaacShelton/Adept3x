use crate::{
    ast::{self, AstWorkspace},
    resolve::{ctx::ResolveCtx, error::ResolveError, job::TypeJob, type_ctx::ResolveTypeCtx},
    resolved::{self, EnumRef, StructureRef, TypeAliasRef},
    workspace::fs::FsNodeId,
};
use std::borrow::Cow;

pub fn resolve_type_jobs(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
    type_jobs: &[TypeJob],
) -> Result<(), ResolveError> {
    for job in type_jobs.iter() {
        let file = ast_workspace
            .files
            .get(&job.physical_file_id)
            .expect("valid ast file");

        let module_file_id = ast_workspace
            .get_owning_module(job.physical_file_id)
            .unwrap_or(job.physical_file_id);

        for (structure_ref, structure) in job.structures.iter().zip(file.structures.iter()) {
            resolve_structure(
                ctx,
                resolved_ast,
                module_file_id,
                job.physical_file_id,
                structure,
                *structure_ref,
            )?;
        }

        for (enum_ref, definition) in job.enums.iter().zip(file.enums.iter()) {
            resolve_enum(
                ctx,
                resolved_ast,
                module_file_id,
                job.physical_file_id,
                definition,
                *enum_ref,
            )?;
        }

        for (type_alias_ref, definition) in job.type_aliases.iter().zip(file.type_aliases.iter()) {
            resolve_type_alias(
                ctx,
                resolved_ast,
                module_file_id,
                job.physical_file_id,
                definition,
                *type_alias_ref,
            )?;
        }
    }

    Ok(())
}

fn resolve_structure(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    structure: &ast::Structure,
    structure_ref: StructureRef,
) -> Result<(), ResolveError> {
    for (field_name, field) in structure.fields.iter() {
        let type_ctx = ResolveTypeCtx::new(
            &resolved_ast,
            module_file_id,
            physical_file_id,
            &ctx.types_in_modules,
        );

        let resolved_type = type_ctx.resolve_or_undeclared(&field.ast_type)?;

        let resolved_struct = resolved_ast
            .structures
            .get_mut(structure_ref)
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

    Ok(())
}

fn resolve_enum(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    definition: &ast::Enum,
    enum_ref: EnumRef,
) -> Result<(), ResolveError> {
    let type_ctx = ResolveTypeCtx::new(
        &resolved_ast,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
    );

    let ast_type = definition
        .backing_type
        .as_ref()
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(ast::TypeKind::u32().at(definition.source)));

    let resolved_type = type_ctx.resolve_or_undeclared(&ast_type)?;
    resolved_ast.enums.get_mut(enum_ref).unwrap().resolved_type = resolved_type;
    Ok(())
}

fn resolve_type_alias(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    definition: &ast::TypeAlias,
    type_alias_ref: TypeAliasRef,
) -> Result<(), ResolveError> {
    let type_ctx = ResolveTypeCtx::new(
        &resolved_ast,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
    );

    let resolved_type = type_ctx.resolve_or_undeclared(&definition.value)?;
    *resolved_ast.type_aliases.get_mut(type_alias_ref).unwrap() = resolved_type;
    Ok(())
}
