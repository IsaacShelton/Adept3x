use super::return_location::ReturnLocation;
use crate::build_llvm_ir::{
    abi::{abi_function::ABIFunction, abi_type::ABITypeKind, has_scalar_evaluation_kind},
    address::Address,
    builder::Builder,
    ctx::{BackendCtx, FunctionSkeleton},
    functions::{
        attribute::{add_param_attribute, create_enum_attribute},
        param_values::{ParamValueConstructionCtx, ParamValues},
        params_mapping::ParamsMapping,
    },
    llvm_type_ref_ext::LLVMTypeRefExt,
    raw_address::RawAddress,
};
use diagnostics::ErrorDiagnostic;
use llvm_sys::{
    core::{LLVMGetParam, LLVMSetValueName2},
    prelude::{LLVMBasicBlockRef, LLVMValueRef},
};

pub struct PrologueInfo {
    pub last_llvm_block: LLVMBasicBlockRef,
    pub param_values: ParamValues,
    pub return_location: Option<ReturnLocation>,
    pub alloca_point: LLVMValueRef,
}

pub fn emit_prologue<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &mut Builder<'env>,
    skeleton: &FunctionSkeleton<'env>,
    abi_function: &ABIFunction<'env>,
    alloca_point: LLVMValueRef,
    entry_basicblock: LLVMBasicBlockRef,
) -> Result<PrologueInfo, ErrorDiagnostic> {
    let ir_function = &ctx.ir_module.funcs[skeleton.ir_func_ref];
    let abi_return_info = &abi_function.return_type;
    let returns_ir_void = ir_function.return_type.is_void();

    builder.position(entry_basicblock);

    let inalloca_subtypes = abi_function
        .inalloca_combined_struct
        .as_ref()
        .map(|inalloca_combined_struct| inalloca_combined_struct.ty.field_types());

    let return_location = (!returns_ir_void)
        .then(|| match &abi_return_info.abi_type.kind {
            ABITypeKind::Indirect(indirect) => ReturnLocation::indirect(
                ctx,
                builder,
                ir_function,
                skeleton.function,
                indirect,
                alloca_point,
            ),
            ABITypeKind::InAlloca(inalloca)
                if !has_scalar_evaluation_kind(&abi_return_info.ir_type) =>
            {
                ReturnLocation::inalloca(
                    ctx,
                    builder,
                    skeleton,
                    inalloca,
                    inalloca_subtypes.as_ref().unwrap().as_slice(),
                )
            }
            _ => ReturnLocation::normal(ctx, builder, alloca_point, &ir_function.return_type),
        })
        .transpose()?;

    let params_mapping =
        ParamsMapping::new(ctx, &ctx.type_layout_cache, abi_function, ctx.ir_module);

    assert_eq!(
        params_mapping.llvm_arity(),
        skeleton.function_type.parameters.len()
    );

    let arg_struct = params_mapping.inalloca_index().map(|inalloca_index| {
        let argument =
            unsafe { LLVMGetParam(skeleton.function, inalloca_index.try_into().unwrap()) };

        Address::from(RawAddress {
            base: argument,
            nullable: false,
            alignment: abi_function
                .inalloca_combined_struct
                .as_ref()
                .unwrap()
                .alignment,
            element_type: abi_function.inalloca_combined_struct.as_ref().unwrap().ty,
        })
    });

    // Mark sret parameter as noalias and rename it for easy reading
    if let Some(sret_index) = params_mapping.sret_index() {
        let argument = unsafe { LLVMGetParam(skeleton.function, sret_index.try_into().unwrap()) };

        let name = c"agg.result";
        unsafe { LLVMSetValueName2(argument, name.as_ptr(), name.count_bytes()) };

        let noalias = create_enum_attribute(c"noalias", 0);
        add_param_attribute(skeleton.function, sret_index, noalias);
    }

    let mut param_values = ParamValues::new();
    let ir_function = &ctx.ir_module.funcs[skeleton.ir_func_ref];

    assert_eq!(abi_function.parameter_types.len(), ir_function.params.len());

    assert_eq!(params_mapping.params().len(), ir_function.params.len());

    for (abi_param, mapped_param) in abi_function
        .parameter_types
        .iter()
        .zip(params_mapping.params())
    {
        let construction_ctx = ParamValueConstructionCtx {
            builder,
            ctx,
            skeleton,
            param_range: mapped_param.range(),
            ir_param_type: &abi_param.ir_type,
            alloca_point,
        };

        match &abi_param.abi_type.kind {
            ABITypeKind::Direct(_) | ABITypeKind::Extend(_) => {
                param_values.push_direct_or_extend(construction_ctx, abi_param)?
            }
            ABITypeKind::Indirect(indirect) => param_values.push_indirect(
                construction_ctx,
                indirect.align,
                indirect.realign,
                false,
            )?,
            ABITypeKind::IndirectAliased(indirect_aliased) => param_values.push_indirect(
                construction_ctx,
                indirect_aliased.align,
                indirect_aliased.realign,
                true,
            )?,
            ABITypeKind::Ignore => param_values.push_ignore(construction_ctx)?,
            ABITypeKind::Expand(_) => param_values.push_expand(construction_ctx)?,
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => param_values
                .push_coerce_and_expand(construction_ctx, coerce_and_expand.coerce_to_type)?,
            ABITypeKind::InAlloca(inalloca) => param_values.push_inalloca(
                construction_ctx,
                inalloca,
                arg_struct.as_ref().unwrap(),
                inalloca_subtypes.as_ref().unwrap().as_slice(),
            )?,
        }
    }

    Ok(PrologueInfo {
        last_llvm_block: builder.current_block(),
        param_values,
        return_location,
        alloca_point,
    })
}
