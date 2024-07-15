pub mod helpers;
mod return_location;
use self::return_location::ReturnLocation;
use crate::llvm_backend::{
    abi::{
        abi_type::{get_struct_field_types, ABITypeKind},
        has_scalar_evaluation_kind,
    },
    address::Address,
    builder::Builder,
    ctx::{BackendCtx, FunctionSkeleton},
    error::BackendError,
    functions::{
        attribute::{add_param_attribute, create_enum_attribute},
        param_values::ParamValues,
        params_mapping::ParamsMapping,
    },
    raw_address::RawAddress,
};
use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMBuildBitCast, LLVMDumpModule, LLVMGetParam, LLVMGetUndef, LLVMInt32Type,
        LLVMPositionBuilderAtEnd, LLVMSetValueName2,
    },
    prelude::LLVMBasicBlockRef,
};

pub struct BackendFnCtx {
    pub return_location: Option<ReturnLocation>,
}

pub fn emit_prologue(
    ctx: &BackendCtx,
    skeleton: &FunctionSkeleton,
    builder: &Builder,
    entry_basicblock: LLVMBasicBlockRef,
) -> Result<Option<BackendFnCtx>, BackendError> {
    let Some(abi_function) = skeleton.abi_function.as_ref() else {
        return Ok(None);
    };

    let abi_return_info = &abi_function.return_type;
    let ir_function = &ctx
        .ir_module
        .functions
        .get(&skeleton.ir_function_ref)
        .unwrap();
    let returns_ir_void = ir_function.return_type.is_void();

    unsafe { LLVMPositionBuilderAtEnd(builder.get(), entry_basicblock) };

    let undef = unsafe { LLVMGetUndef(LLVMInt32Type()) };
    let alloca_point = unsafe {
        LLVMBuildBitCast(
            builder.get(),
            undef,
            LLVMInt32Type(),
            cstr!("allocapt").as_ptr(),
        )
    };

    let inalloca_subtypes = abi_function
        .inalloca_combined_struct
        .as_ref()
        .map(|inalloca_combined_struct| get_struct_field_types(inalloca_combined_struct.ty));

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
            _ => ReturnLocation::normal(builder, ctx, alloca_point, &ir_function.return_type),
        })
        .transpose()?;

    let params_mapping = ParamsMapping::new(&ctx.type_layout_cache, abi_function, ctx.ir_module);

    assert_eq!(
        params_mapping.llvm_arity(),
        skeleton.function_type.parameters.len()
    );

    let arg_struct = params_mapping.inalloca_index().map(|inalloca_index| {
        let argument =
            unsafe { LLVMGetParam(skeleton.function, inalloca_index.try_into().unwrap()) };

        Address {
            base: RawAddress {
                base: argument,
                nullable: false,
                alignment: abi_function
                    .inalloca_combined_struct
                    .as_ref()
                    .unwrap()
                    .alignment,
                element_type: abi_function.inalloca_combined_struct.as_ref().unwrap().ty,
            },
            offset: None,
        }
    });

    // Mark sret parameter as noalias and rename it for easy reading
    if let Some(sret_index) = params_mapping.sret_index() {
        let argument = unsafe { LLVMGetParam(skeleton.function, sret_index.try_into().unwrap()) };

        let name = cstr!("agg.result");
        unsafe { LLVMSetValueName2(argument, name.as_ptr(), name.count_bytes()) };

        let noalias = create_enum_attribute(cstr!("noalias"), 0);
        add_param_attribute(skeleton.function, sret_index, noalias);
    }

    let mut param_values = ParamValues::new();

    let ir_function = ctx
        .ir_module
        .functions
        .get(&skeleton.ir_function_ref)
        .unwrap();

    assert_eq!(
        abi_function.parameter_types.len(),
        ir_function.parameters.len()
    );

    assert_eq!(params_mapping.params().len(), ir_function.parameters.len());

    for (abi_param, mapped_param) in abi_function
        .parameter_types
        .iter()
        .zip(params_mapping.params())
    {
        let ty = &abi_param.ir_type;
        let llvm_param_range = mapped_param.range();

        match &abi_param.abi_type.kind {
            ABITypeKind::Direct(_) => todo!(),
            ABITypeKind::Extend(_) => todo!(),
            ABITypeKind::Indirect(indirect) => param_values.push_indirect(
                builder,
                ctx,
                skeleton,
                llvm_param_range,
                ty,
                indirect.align,
                indirect.realign,
                false,
                alloca_point,
            )?,
            ABITypeKind::IndirectAliased(indirect_aliased) => param_values.push_indirect(
                builder,
                ctx,
                skeleton,
                llvm_param_range,
                ty,
                indirect_aliased.align,
                indirect_aliased.realign,
                true,
                alloca_point,
            )?,
            ABITypeKind::Ignore => {
                param_values.push_ignore(builder, ctx, llvm_param_range, ty, alloca_point)?
            }
            ABITypeKind::Expand(_) => todo!(),
            ABITypeKind::CoerceAndExpand(_) => todo!(),
            ABITypeKind::InAlloca(inalloca) => param_values.push_inalloca(
                builder,
                ctx,
                inalloca,
                llvm_param_range,
                arg_struct.as_ref().unwrap(),
                ty,
                &ctx.type_layout_cache,
                inalloca_subtypes.as_ref().unwrap().as_slice(),
            )?,
        }
    }

    unsafe { LLVMDumpModule(ctx.backend_module.get()) };
    unimplemented!();
}
