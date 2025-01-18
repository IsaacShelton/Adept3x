use super::lower_expr;
use crate::{
    asg::{self, Asg, Call, Expr, PolyCall, TypedExpr},
    ir,
    lower::{builder::Builder, datatype::lower_type, error::LowerError, function::lower_func_head},
    resolve::{PolyRecipe, PolyValue},
    source_files::Source,
};
use indexmap::IndexMap;
use std::borrow::Borrow;

pub fn lower_expr_call(
    builder: &mut Builder,
    ir_module: &ir::Module,
    expr: &Expr,
    function: &asg::Func,
    asg: &Asg,
    call: &Call,
) -> Result<ir::Value, LowerError> {
    lower_expr_call_core(
        builder,
        ir_module,
        expr,
        function,
        asg,
        call.callee.function,
        &call.callee.recipe,
        call.args.as_slice(),
    )
}

pub fn lower_expr_poly_call(
    builder: &mut Builder,
    ir_module: &ir::Module,
    expr: &Expr,
    callee_func: &asg::Func,
    asg: &Asg,
    poly_call: &PolyCall,
) -> Result<ir::Value, LowerError> {
    let impl_ref = builder
        .poly_recipe()
        .resolve_impl(&poly_call.callee.polymorph, expr.source)
        .map_err(LowerError::from)?;

    let imp = asg.impls.get(impl_ref).expect("referenced impl to exist");

    let func_ref = imp
        .body
        .get(&poly_call.callee.member)
        .expect("expected impl body function referenced by poly call to exist");

    lower_expr_call_core(
        builder,
        ir_module,
        expr,
        callee_func,
        asg,
        *func_ref,
        &poly_call.callee.recipe,
        poly_call.args.as_slice(),
    )
}

fn lower_expr_call_core(
    builder: &mut Builder,
    ir_module: &ir::Module,
    expr: &Expr,
    function: &asg::Func,
    asg: &Asg,
    func_ref: asg::FuncRef,
    callee_recipe: &PolyRecipe,
    all_args: &[TypedExpr],
) -> Result<ir::Value, LowerError> {
    let callee = asg
        .funcs
        .get(func_ref)
        .expect("referenced function to exist");

    let args = all_args
        .iter()
        .map(|arg| lower_expr(builder, ir_module, &arg.expr, function, asg))
        .collect::<Result<Box<[_]>, _>>()?;

    let variadic_arg_types = all_args[callee.params.required.len()..]
        .iter()
        .map(|arg| lower_type(ir_module, &builder.unpoly(&arg.ty)?, asg))
        .collect::<Result<Box<[_]>, _>>()?;

    let recipe = lower_call_poly_recipe(builder, &callee_recipe, expr.source)?;

    let function = ir_module.funcs.translate(func_ref, &recipe, || {
        lower_func_head(ir_module, func_ref, &recipe, asg)
    })?;

    Ok(builder.push(ir::Instr::Call(ir::Call {
        func: function,
        args,
        unpromoted_variadic_arg_types: variadic_arg_types,
    })))
}

fn lower_call_poly_recipe(
    builder: &Builder,
    callee_recipe: &PolyRecipe,
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
                todo!("compile-time expression parameters are not supported when calling generic functions yet")
            }
            PolyValue::Impl(impl_ref) => {
                polymorphs.insert(name.clone(), PolyValue::Impl(*impl_ref));
            }
            PolyValue::PolyImpl(from_name) => {
                let Some(poly_value) = builder.poly_recipe().polymorphs.get(from_name) else {
                    return Err(LowerError::other(
                        format!("Undefined polymorph '{}' cannot be used as polymorphic trait implementation", name),
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
