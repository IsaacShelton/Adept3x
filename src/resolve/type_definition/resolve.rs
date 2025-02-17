use crate::{
    asg::{self, Asg, EnumRef, StructRef, TraitFunc, TraitRef, Type, TypeAliasRef},
    ast::{self, AstWorkspace, TypeParams},
    resolve::{
        ctx::ResolveCtx,
        error::{ResolveError, ResolveErrorKind},
        func_head::resolve_parameters,
        job::TypeJob,
        type_ctx::{ResolveTypeCtx, ResolveTypeOptions},
        PolymorphErrorKind,
    },
    workspace::fs::FsNodeId,
};
use indexmap::IndexMap;
use std::borrow::Cow;

pub fn resolve_type_jobs(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
    type_jobs: &[TypeJob],
) -> Result<(), ResolveError> {
    for job in type_jobs.iter() {
        let file = ast_workspace
            .files
            .get(&job.physical_file_id)
            .expect("valid ast file");

        let module_file_id = ast_workspace.get_owning_module_or_self(job.physical_file_id);

        for (trait_ref, user_trait) in job.traits.iter().zip(file.traits.iter()) {
            resolve_trait(
                ctx,
                asg,
                module_file_id,
                job.physical_file_id,
                user_trait,
                *trait_ref,
            )?;
        }

        for (struct_ref, structure) in job.structs.iter().zip(file.structs.iter()) {
            resolve_structure(
                ctx,
                asg,
                module_file_id,
                job.physical_file_id,
                structure,
                *struct_ref,
            )?;
        }

        for (enum_ref, definition) in job.enums.iter().zip(file.enums.iter()) {
            resolve_enum(
                ctx,
                asg,
                module_file_id,
                job.physical_file_id,
                definition,
                *enum_ref,
            )?;
        }

        for (type_alias_ref, definition) in job.type_aliases.iter().zip(file.type_aliases.iter()) {
            resolve_type_alias(
                ctx,
                asg,
                module_file_id,
                job.physical_file_id,
                definition,
                *type_alias_ref,
            )?;
        }
    }

    Ok(())
}

pub fn ensure_declared_polymorphs(ty: &Type, params: &TypeParams) -> Result<(), ResolveError> {
    let mut ok = Ok(());

    ty.kind.for_each_polymorph(&mut |name| {
        if params.names().filter(|n| *n == name).next().is_none() && ok.is_ok() {
            ok = Err(
                ResolveErrorKind::from(PolymorphErrorKind::UndefinedPolymorph(name.to_string()))
                    .at(ty.source),
            );
        }
    });

    ok
}

fn resolve_structure(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    structure: &ast::Struct,
    struct_ref: StructRef,
) -> Result<(), ResolveError> {
    for (field_name, field) in structure.fields.iter() {
        let type_ctx = ResolveTypeCtx::new(
            &asg,
            module_file_id,
            physical_file_id,
            &ctx.types_in_modules,
        );

        let ty =
            type_ctx.resolve_or_undeclared(&field.ast_type, ResolveTypeOptions::KeepAliases)?;

        ensure_declared_polymorphs(&ty, &structure.params)?;

        let resolved_struct = asg.structs.get_mut(struct_ref).expect("valid struct");

        resolved_struct.fields.insert(
            field_name.clone(),
            asg::Field {
                ty,
                privacy: field.privacy,
                source: field.source,
            },
        );
    }

    Ok(())
}

fn resolve_enum(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    definition: &ast::Enum,
    enum_ref: EnumRef,
) -> Result<(), ResolveError> {
    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
    );

    let ast_type = definition
        .backing_type
        .as_ref()
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(ast::TypeKind::u32().at(definition.source)));

    let ty = type_ctx.resolve_or_undeclared(&ast_type, ResolveTypeOptions::KeepAliases)?;
    asg.enums.get_mut(enum_ref).unwrap().ty = ty;
    Ok(())
}

fn resolve_type_alias(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    definition: &ast::TypeAlias,
    type_alias_ref: TypeAliasRef,
) -> Result<(), ResolveError> {
    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
    );

    let params = &asg.type_aliases.get(type_alias_ref).unwrap().params;
    let ty = type_ctx.resolve_or_undeclared(&definition.value, ResolveTypeOptions::KeepAliases)?;

    ensure_declared_polymorphs(&ty, params)?;

    asg.type_aliases.get_mut(type_alias_ref).unwrap().becomes = ty;
    Ok(())
}

fn resolve_trait(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    definition: &ast::Trait,
    trait_ref: TraitRef,
) -> Result<(), ResolveError> {
    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
    );

    let mut funcs = IndexMap::new();

    for func in &definition.funcs {
        let params = resolve_parameters(&type_ctx, &func.params)?;
        let return_type = type_ctx.resolve(&func.return_type, ResolveTypeOptions::KeepAliases)?;

        if funcs
            .insert(
                func.name.clone(),
                TraitFunc {
                    params,
                    return_type,
                    source: func.source,
                },
            )
            .is_some()
        {
            return Err(ResolveError::other(
                format!(
                    "Cannot have multiple functions named '{}' within trait",
                    &func.name
                ),
                func.source,
            ));
        }
    }

    asg.traits.get_mut(trait_ref).unwrap().funcs = funcs;
    Ok(())
}
