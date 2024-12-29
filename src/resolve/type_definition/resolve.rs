use crate::{
    asg::{
        self, Asg, CurrentConstraints, EnumRef, StructRef, TraitFunction, TraitRef, TypeAliasRef,
    },
    ast::{self, AstWorkspace},
    resolve::{
        ctx::ResolveCtx,
        error::ResolveError,
        function_head::resolve_parameters,
        job::TypeJob,
        type_ctx::{resolve_constraints, ResolveTypeCtx},
    },
    workspace::fs::FsNodeId,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

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

        for (struct_ref, structure) in job.structures.iter().zip(file.structs.iter()) {
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

fn resolve_structure(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    structure: &ast::Struct,
    struct_ref: StructRef,
) -> Result<(), ResolveError> {
    for (field_name, field) in structure.fields.iter() {
        let pre_constraints = CurrentConstraints::new_empty();
        let pre_type_ctx = ResolveTypeCtx::new(
            &asg,
            module_file_id,
            physical_file_id,
            &ctx.types_in_modules,
            &pre_constraints,
        );

        let mut constraints = HashMap::new();
        for (name, parameter) in structure.parameters.iter() {
            constraints.insert(
                name.into(),
                HashSet::from_iter(
                    resolve_constraints(&pre_type_ctx, &parameter.constraints)?.drain(..),
                ),
            );
        }

        let constraints = CurrentConstraints::new(constraints);

        let type_ctx = ResolveTypeCtx::new(
            &asg,
            module_file_id,
            physical_file_id,
            &ctx.types_in_modules,
            &constraints,
        );

        let ty = type_ctx.resolve_or_undeclared(&field.ast_type)?;

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
    let constraints = CurrentConstraints::new_empty();
    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
        &constraints,
    );

    let ast_type = definition
        .backing_type
        .as_ref()
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(ast::TypeKind::u32().at(definition.source)));

    let ty = type_ctx.resolve_or_undeclared(&ast_type)?;
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
    let constraints = CurrentConstraints::new_empty();
    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
        &constraints,
    );

    let ty = type_ctx.resolve_or_undeclared(&definition.value)?;
    *asg.type_aliases.get_mut(type_alias_ref).unwrap() = ty;
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
    let constraints = CurrentConstraints::new_empty();
    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
        &constraints,
    );

    let mut functions = Vec::with_capacity(definition.funcs.len());

    for function in &definition.funcs {
        let parameters = resolve_parameters(&type_ctx, &function.params)?;
        let return_type = type_ctx.resolve(&function.return_type)?;

        functions.push(TraitFunction {
            name: function.name.clone(),
            parameters,
            return_type,
            source: function.source,
        });
    }

    asg.traits.get_mut(trait_ref).unwrap().functions = functions;
    Ok(())
}
