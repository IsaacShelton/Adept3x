use super::{
    builder::{unpoly, Builder},
    datatype::lower_type,
    error::{LowerError, LowerErrorKind},
    stmts::lower_stmts,
};
use crate::{
    asg::{self, Asg},
    ir::{self, BasicBlocks, Literal},
    resolve::PolyRecipe,
    tag::Tag,
};

pub fn lower_func_body(
    ir_module: &ir::Module,
    func_ref: asg::FuncRef,
    poly_recipe: &PolyRecipe,
    asg: &Asg,
) -> Result<BasicBlocks, LowerError> {
    let func = asg.funcs.get(func_ref).expect("valid function reference");

    if func.is_foreign {
        return Ok(BasicBlocks::new());
    }
    let mut builder = Builder::new_with_starting_block(poly_recipe);

    // Allocate parameters
    let param_vars = func
        .vars
        .instances
        .iter()
        .take(func.vars.num_params)
        .map(|instance| {
            Ok(builder.push(ir::Instr::Alloca(lower_type(
                ir_module,
                &builder.unpoly(&instance.ty)?,
                asg,
            )?)))
        })
        .collect::<Result<Vec<_>, LowerError>>()?;

    // Allocate non-parameter stack variables
    for var in func.vars.instances.iter().skip(func.vars.num_params) {
        builder.push(ir::Instr::Alloca(lower_type(
            ir_module,
            &builder.unpoly(&var.ty)?,
            asg,
        )?));
    }

    for (i, destination) in param_vars.into_iter().enumerate() {
        let source = builder.push(ir::Instr::Parameter(i.try_into().unwrap()));

        builder.push(ir::Instr::Store(ir::Store {
            new_value: source,
            destination,
        }));
    }

    lower_stmts(&mut builder, ir_module, &func.stmts, func, asg)?;

    if !builder.is_block_terminated() {
        if func.return_type.kind.is_void() {
            if func.tag == Some(Tag::Main) && !builder.is_block_terminated() {
                builder.push(ir::Instr::Return(Some(ir::Value::Literal(
                    Literal::Signed32(0),
                ))));
            } else {
                builder.terminate();
            }
        } else {
            return Err(LowerErrorKind::MustReturnValueOfTypeBeforeExitingFunction {
                return_type: func.return_type.to_string(),
                function: func.name.display(&asg.workspace.fs).to_string(),
            }
            .at(func.source));
        }
    }

    Ok(builder.build())
}

pub fn lower_func_head(
    ir_module: &ir::Module,
    func_ref: asg::FuncRef,
    poly_recipe: &PolyRecipe,
    asg: &Asg,
) -> Result<ir::FuncRef, LowerError> {
    let func = asg.funcs.get(func_ref).expect("valid function reference");
    let basicblocks = BasicBlocks::default();

    let mut params = vec![];
    for param in func.params.required.iter() {
        params.push(lower_type(
            ir_module,
            &unpoly(poly_recipe, &param.ty)?,
            asg,
        )?);
    }

    let mut return_type = lower_type(ir_module, &unpoly(poly_recipe, &func.return_type)?, asg)?;

    if func.tag == Some(Tag::Main) {
        if let ir::Type::Void = return_type {
            return_type = ir::Type::S32;
        }
    }

    let mangled_name = if func.name.plain() == "main" {
        "main".into()
    } else if func.is_foreign || func.is_exposed {
        func.name.plain().to_string()
    } else {
        func.name.display(&asg.workspace.fs).to_string() + &poly_recipe.to_string()
    };

    let is_main = mangled_name == "main";
    let is_exposed = func.is_exposed || is_main;

    Ok(ir_module.funcs.insert(
        func_ref,
        ir::Func {
            mangled_name,
            basicblocks,
            params,
            return_type,
            is_cstyle_variadic: func.params.is_cstyle_vararg,
            is_foreign: func.is_foreign,
            is_exposed,
            abide_abi: func.abide_abi && ir_module.target.arch().is_some(),
        },
    ))
}
