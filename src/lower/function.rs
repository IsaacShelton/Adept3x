use super::{
    builder::Builder,
    error::{LowerError, LowerErrorKind},
    lower_type,
    stmts::lower_stmts,
};
use crate::{
    ir::{self, BasicBlocks, Literal},
    resolved,
    tag::Tag,
};

pub fn lower_function(
    ir_module: &mut ir::Module,
    function_ref: resolved::FunctionRef,
    function: &resolved::Function,
    resolved_ast: &resolved::Ast,
) -> Result<(), LowerError> {
    let basicblocks = if !function.is_foreign {
        let mut builder = Builder::new_with_starting_block();

        // Allocate parameters
        let parameter_variables = function
            .variables
            .instances
            .iter()
            .take(function.variables.num_parameters)
            .map(|instance| {
                Ok(builder.push(ir::Instruction::Alloca(lower_type(
                    &ir_module.target,
                    &instance.resolved_type,
                    resolved_ast,
                )?)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Allocate non-parameter stack variables
        for variable_instance in function
            .variables
            .instances
            .iter()
            .skip(function.variables.num_parameters)
        {
            builder.push(ir::Instruction::Alloca(lower_type(
                &ir_module.target,
                &variable_instance.resolved_type,
                resolved_ast,
            )?));
        }

        for (i, destination) in parameter_variables.into_iter().enumerate() {
            let source = builder.push(ir::Instruction::Parameter(i.try_into().unwrap()));

            builder.push(ir::Instruction::Store(ir::Store {
                new_value: source,
                destination,
            }));
        }

        lower_stmts(
            &mut builder,
            ir_module,
            &function.stmts,
            function,
            resolved_ast,
        )?;

        if !builder.is_block_terminated() {
            if function.return_type.kind.is_void() {
                if function.tag == Some(Tag::Main) && !builder.is_block_terminated() {
                    builder.push(ir::Instruction::Return(Some(ir::Value::Literal(
                        Literal::Signed32(0),
                    ))));
                } else {
                    builder.terminate();
                }
            } else {
                return Err(LowerErrorKind::MustReturnValueOfTypeBeforeExitingFunction {
                    return_type: function.return_type.to_string(),
                    function: function
                        .name
                        .display(&resolved_ast.workspace.fs)
                        .to_string(),
                }
                .at(function.source));
            }
        }

        builder.build()
    } else {
        BasicBlocks::default()
    };

    let mut parameters = vec![];
    for parameter in function.parameters.required.iter() {
        parameters.push(lower_type(
            &ir_module.target,
            &parameter.resolved_type,
            resolved_ast,
        )?);
    }

    let mut return_type = lower_type(&ir_module.target, &function.return_type, resolved_ast)?;

    if function.tag == Some(Tag::Main) {
        if let ir::Type::Void = return_type {
            return_type = ir::Type::S32;
        }
    }

    let mangled_name = if function.name.plain() == "main" {
        "main".into()
    } else if function.is_foreign {
        function.name.plain().to_string()
    } else {
        function
            .name
            .display(&resolved_ast.workspace.fs)
            .to_string()
    };

    let is_main = mangled_name == "main";
    let is_exposed = is_main;

    // Ensure `main` is referenced
    if is_exposed {
        ir_module
            .function_uses
            .write()
            .unwrap()
            .insert(function_ref);
    }

    ir_module.functions.insert(
        function_ref,
        ir::Function {
            mangled_name,
            basicblocks,
            parameters,
            return_type,
            is_cstyle_variadic: function.parameters.is_cstyle_vararg,
            is_foreign: function.is_foreign,
            is_exposed,
            abide_abi: function.abide_abi && ir_module.target.arch().is_some(),
        },
    );

    Ok(())
}