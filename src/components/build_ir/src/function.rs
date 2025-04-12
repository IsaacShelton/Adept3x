use super::{
    datatype::lower_type,
    error::{LowerError, LowerErrorKind},
    func_builder::{FuncBuilder, unpoly},
};
use crate::ModBuilder;
use asg::{Asg, PolyRecipe};
use attributes::{Exposure, SymbolOwnership, Tag};
use ir::{self, BasicBlocks, Literal};

pub fn lower_func_body(
    mod_builder: &ModBuilder,
    func_ref: asg::FuncRef,
    poly_recipe: &PolyRecipe,
    asg: &Asg,
) -> Result<BasicBlocks, LowerError> {
    let func = &asg.funcs[func_ref];

    if !func.ownership.is_owned() {
        return Ok(BasicBlocks::new());
    }

    let mut func_builder = FuncBuilder::new_with_starting_block(mod_builder, poly_recipe, func);

    // Allocate parameters
    let param_vars = func
        .vars
        .instances
        .iter()
        .take(func.vars.num_params)
        .map(|instance| {
            Ok(func_builder.push(ir::Instr::Alloca(func_builder.lower_type(&instance.ty)?)))
        })
        .collect::<Result<Vec<_>, LowerError>>()?;

    // Allocate non-parameter stack variables
    for var in func.vars.instances.iter().skip(func.vars.num_params) {
        func_builder.push(ir::Instr::Alloca(func_builder.lower_type(&var.ty)?));
    }

    for (i, destination) in param_vars.into_iter().enumerate() {
        let new_value = func_builder.push(ir::Instr::Parameter(i.try_into().unwrap()));

        func_builder.push(ir::Instr::Store(ir::Store {
            new_value,
            destination,
        }));
    }

    func_builder.lower_stmts(&func.stmts)?;

    if !func_builder.is_block_terminated() {
        if !func.return_type.is_void() {
            return Err(LowerErrorKind::MustReturnValueOfTypeBeforeExitingFunction {
                return_type: func.return_type.to_string(),
                function: func.name.display(&asg.workspace.fs).to_string(),
            }
            .at(func.source));
        }

        if func.tag == Some(Tag::Main) {
            func_builder.push(ir::Instr::Return(Some(Literal::Signed32(0).into())));
        } else {
            func_builder.terminate();
        }
    }

    Ok(func_builder.build())
}

pub fn lower_func_head(
    mod_builder: &ModBuilder,
    func_ref: asg::FuncRef,
    poly_recipe: &PolyRecipe,
) -> Result<ir::FuncRef, LowerError> {
    let func = &mod_builder.asg.funcs[func_ref];
    let basicblocks = BasicBlocks::default();

    let mut params = vec![];
    for param in func.params.required.iter() {
        params.push(lower_type(mod_builder, &unpoly(poly_recipe, &param.ty)?)?);
    }

    let mut return_type = lower_type(mod_builder, &unpoly(poly_recipe, &func.return_type)?)?;

    if func.tag == Some(Tag::Main) {
        if let ir::Type::Void = return_type {
            return_type = ir::Type::S32;
        }
    }

    let mangled_name = if func.tag == Some(Tag::Main) {
        "main".into()
    } else if func.ownership.should_mangle() {
        func.name.display(&mod_builder.asg.workspace.fs).to_string() + &poly_recipe.to_string()
    } else {
        func.name.plain().to_string()
    };

    let ownership = if func.tag == Some(Tag::Main) {
        SymbolOwnership::Owned(Exposure::Exposed)
    } else {
        func.ownership
    };

    Ok(mod_builder.funcs.insert(
        func_ref,
        ir::Func {
            mangled_name,
            basicblocks,
            params,
            return_type,
            is_cstyle_variadic: func.params.is_cstyle_vararg,
            ownership,
            abide_abi: func.abide_abi && mod_builder.target.arch().is_some(),
        },
    ))
}
