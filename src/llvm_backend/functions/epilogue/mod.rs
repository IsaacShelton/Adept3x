use super::{
    helpers::{build_coerced_load, get_natural_type_alignment},
    return_location::ReturnLocation,
};
use crate::{
    backend::BackendError,
    llvm_backend::{
        abi::{
            abi_type::{is_padding_for_coerce_expand, ABITypeKind},
            has_scalar_evaluation_kind,
        },
        address::Address,
        backend_type::to_backend_type,
        builder::{Builder, Volatility},
        ctx::{BackendCtx, FunctionSkeleton},
        functions::helpers::emit_address_at_offset,
        llvm_type_ref_ext::LLVMTypeRefExt,
        raw_address::RawAddress,
    },
};
use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMBuildInsertValue, LLVMBuildRet, LLVMBuildRetVoid, LLVMBuildStructGEP2,
        LLVMGetLastParam, LLVMGetParam, LLVMGetPoison,
    },
    prelude::{LLVMBasicBlockRef, LLVMValueRef},
};
use std::ptr::null_mut;

pub struct EpilogueInfo {
    pub llvm_basicblock: LLVMBasicBlockRef,
}

pub fn emit_epilogue(
    ctx: &BackendCtx,
    builder: &Builder,
    skeleton: &FunctionSkeleton,
    epilogue_basicblock: LLVMBasicBlockRef,
    return_location: Option<&ReturnLocation>,
    alloca_point: LLVMValueRef,
) -> Result<EpilogueInfo, BackendError> {
    builder.position(epilogue_basicblock);

    let abi_function = skeleton
        .abi_function
        .as_ref()
        .expect("should have ABI function information for epilogue");

    let Some(return_location) = return_location else {
        unsafe { LLVMBuildRetVoid(builder.get()) };

        return Ok(EpilogueInfo {
            llvm_basicblock: epilogue_basicblock,
        });
    };

    let ir_function = ctx
        .ir_module
        .functions
        .get(&skeleton.ir_function_ref)
        .unwrap();

    let ir_return_type = &ir_function.return_type;
    let abi_return_info = &abi_function.return_type;

    let return_value = match &abi_return_info.abi_type.kind {
        ABITypeKind::Direct(_) | ABITypeKind::Extend(_) => {
            let abi_type = &abi_return_info.abi_type;
            let coerce_to_type = abi_type.coerce_to_type().flatten().unwrap();

            let value = emit_address_at_offset(
                builder,
                ctx.target_data,
                &abi_return_info.abi_type,
                &return_location.return_value_address,
            );

            Some(build_coerced_load(
                ctx,
                builder,
                &value,
                coerce_to_type,
                alloca_point,
            ))
        }
        ABITypeKind::Indirect(indirect) => {
            let return_pointer =
                unsafe { LLVMGetParam(skeleton.function, indirect.sret_position().into()) };

            if has_scalar_evaluation_kind(ir_return_type) {
                let alignment = get_natural_type_alignment(&ctx.type_layout_cache, ir_return_type);

                let address = Address::from(RawAddress {
                    base: return_pointer,
                    nullable: false,
                    alignment,
                    element_type: unsafe {
                        to_backend_type(ctx.for_making_type(), ir_return_type)?
                    },
                });

                Some(builder.load(&address, Volatility::Normal))
            } else if ir_return_type.is_complex() {
                todo!("returning complex types via indirect ABI return mode not supported yet")
            } else {
                // Nothing to do for composite-like types
                None
            }
        }
        ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
            let coerce_to_type = coerce_and_expand.coerce_to_type;
            let field_types = coerce_to_type.field_types();

            let address = return_location
                .return_value_address
                .with_element_type(coerce_to_type);

            let mut field_values = Vec::with_capacity(field_types.len());

            for (field_i, coerced_element_type) in field_types.iter().copied().enumerate() {
                if is_padding_for_coerce_expand(coerced_element_type) {
                    continue;
                }

                let element_address = builder.gep_struct(
                    ctx.target_data,
                    &address,
                    field_i,
                    Some(field_types.as_slice()),
                );

                field_values.push(builder.load(&element_address, Volatility::Normal));
            }

            if field_values.len() == 1 {
                Some(field_values[0])
            } else {
                let return_type = coerce_and_expand.unpadded_coerce_and_expand_type;
                let mut aggregate_return_value = unsafe { LLVMGetPoison(return_type) };

                for (field_i, element) in field_values.iter().copied().enumerate() {
                    aggregate_return_value = unsafe {
                        LLVMBuildInsertValue(
                            builder.get(),
                            aggregate_return_value,
                            element,
                            field_i.try_into().unwrap(),
                            cstr!("").as_ptr(),
                        )
                    };
                }

                Some(aggregate_return_value)
            }
        }
        ABITypeKind::InAlloca(inalloca) => {
            assert!(has_scalar_evaluation_kind(ir_return_type));

            inalloca.sret.then(|| {
                let arg_struct = unsafe { LLVMGetLastParam(skeleton.function) };
                let arg_struct_type = abi_function.inalloca_combined_struct.as_ref().unwrap().ty;

                let field_type =
                    arg_struct_type.field_types()[inalloca.alloca_field_index as usize];

                let sret = unsafe {
                    LLVMBuildStructGEP2(
                        builder.get(),
                        arg_struct_type,
                        arg_struct,
                        inalloca.alloca_field_index,
                        cstr!("").as_ptr(),
                    )
                };

                builder.load_aligned(
                    field_type,
                    sret,
                    ctx.ir_module.target_info.pointer_layout().alignment,
                    Volatility::Normal,
                    cstr!("sret"),
                )
            })
        }
        ABITypeKind::Ignore => None,
        ABITypeKind::Expand(_) | ABITypeKind::IndirectAliased(_) => {
            panic!("invalid ABI return mode")
        }
    };

    // Return the proper value for the ABI (potentially void)
    unsafe { LLVMBuildRet(builder.get(), return_value.unwrap_or_else(null_mut)) };

    Ok(EpilogueInfo {
        llvm_basicblock: epilogue_basicblock,
    })
}
