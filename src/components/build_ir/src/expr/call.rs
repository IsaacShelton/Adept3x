use super::lower_expr;
use crate::{error::LowerError, func_builder::FuncBuilder, function::lower_func_head};
use asg::{IntoPolyRecipeResolver, PolyRecipe, PolyValue};
use indexmap::IndexMap;
use source_files::Source;
use std::borrow::Borrow;

pub fn lower_expr_call(
    builder: &mut FuncBuilder,
    expr: &asg::Expr,
    call: &asg::Call,
) -> Result<ir::Value, LowerError> {
    lower_expr_call_core(
        builder,
        expr,
        call.callee.func_ref,
        &call.callee.recipe,
        call.args.as_slice(),
    )
}

pub fn lower_expr_poly_call(
    builder: &mut FuncBuilder,
    expr: &asg::Expr,
    poly_call: &asg::PolyCall,
) -> Result<ir::Value, LowerError> {
    let impl_ref = builder
        .poly_recipe()
        .resolver()
        .resolve_impl(&poly_call.callee.polymorph, expr.source)
        .map_err(LowerError::from)?;

    let imp = &builder.asg().impls[impl_ref];

    let func_ref = imp
        .body
        .get(&poly_call.callee.member)
        .expect("expected impl body function referenced by poly call to exist");

    lower_expr_call_core(
        builder,
        expr,
        *func_ref,
        &poly_call.callee.recipe,
        poly_call.args.as_slice(),
    )
}

fn lower_expr_call_core(
    builder: &mut FuncBuilder,
    expr: &asg::Expr,
    callee_func_ref: asg::FuncRef,
    callee_recipe: &asg::PolyRecipe,
    all_args: &[asg::TypedExpr],
) -> Result<ir::Value, LowerError> {
    let args = all_args
        .iter()
        .map(|arg| lower_expr(builder, &arg.expr))
        .collect::<Result<Box<[_]>, _>>()?;

    let callee = &builder.asg().funcs[callee_func_ref];

    let variadic_arg_types = all_args[callee.params.required.len()..]
        .iter()
        .map(|arg| builder.lower_type(&arg.ty))
        .collect::<Result<Box<[_]>, _>>()?;

    let recipe = lower_call_poly_recipe(builder, &callee_recipe, expr.source)?;

    let func =
        builder
            .mod_builder()
            .funcs
            .translate(builder.asg(), callee_func_ref, &recipe, || {
                lower_func_head(builder.mod_builder(), callee_func_ref, &recipe)
            })?;

    Ok(builder.push(ir::Instr::Call(ir::Call {
        func,
        args,
        unpromoted_variadic_arg_types: variadic_arg_types,
    })))
}

fn lower_call_poly_recipe(
    builder: &FuncBuilder,
    callee_recipe: &asg::PolyRecipe,
    source: Source,
) -> Result<PolyRecipe, LowerError> {
    let mut polymorphs = IndexMap::<String, PolyValue>::new();

    for (name, value) in callee_recipe.polymorphs.iter() {
        match value {
            PolyValue::Type(ty) => {
                polymorphs.insert(
                    name.clone(),
                    PolyValue::Type(Borrow::<asg::Type>::borrow(&builder.unpoly(&ty)?.0).clone()),
                );
            }
            PolyValue::Expr(_) => {
                todo!(
                    "compile-time expression parameters are not supported when calling generic functions yet"
                )
            }
            PolyValue::Impl(impl_ref) => {
                polymorphs.insert(name.clone(), PolyValue::Impl(*impl_ref));
            }
            PolyValue::PolyImpl(from_name) => {
                let Some(poly_value) = builder.poly_recipe().polymorphs.get(from_name) else {
                    return Err(LowerError::other(
                        format!(
                            "Undefined polymorph '{}' cannot be used as polymorphic trait implementation",
                            name
                        ),
                        source,
                    ));
                };

                let PolyValue::Impl(impl_ref) = poly_value else {
                    return Err(LowerError::other(
                        format!("Polymorph '{}' must be a trait implementation", name),
                        source,
                    ));
                };

                polymorphs.insert(name.clone(), PolyValue::Impl(*impl_ref));
            }
        }
    }

    Ok(PolyRecipe { polymorphs })
}
