use super::block::create_function_block;
use super::prologue::emit_prologue;
use crate::llvm_backend::{
    builder::Builder, ctx::BackendCtx, error::BackendError, value_catalog::ValueCatalog,
    values::build_value,
};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMAddIncoming, LLVMAppendBasicBlock},
    prelude::LLVMBasicBlockRef,
};
use std::cell::OnceCell;

pub unsafe fn create_function_bodies(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (ir_function_ref, skeleton) in ctx.func_skeletons.iter() {
        if let Some(ir_function) = ctx.ir_module.functions.get(ir_function_ref) {
            let mut builder = Builder::new();
            let mut value_catalog = ValueCatalog::new(ir_function.basicblocks.len());

            let basicblocks = ir_function
                .basicblocks
                .iter()
                .enumerate()
                .map(|(id, ir_basicblock)| {
                    (
                        id,
                        ir_basicblock,
                        LLVMAppendBasicBlock(skeleton.function, cstr!("").as_ptr()),
                    )
                })
                .collect::<Vec<_>>();

            let overflow_basicblock: OnceCell<LLVMBasicBlockRef> = OnceCell::new();

            if ir_function.abide_abi {
                if let Some((_, _, llvm_entry_basicblock)) = basicblocks.first() {
                    emit_prologue(ctx, skeleton, &builder, *llvm_entry_basicblock);
                }
            }

            for (ir_basicblock_id, ir_basicblock, llvm_basicblock) in basicblocks.iter() {
                create_function_block(
                    ctx,
                    &mut value_catalog,
                    &overflow_basicblock,
                    &builder,
                    *ir_basicblock_id,
                    ir_basicblock,
                    *llvm_basicblock,
                    skeleton.function,
                    &basicblocks,
                )?;
            }

            for relocation in builder.take_phi_relocations().iter() {
                for incoming in relocation.incoming.iter() {
                    let (_, _, backend_block) = basicblocks
                        .get(incoming.basicblock_id)
                        .expect("backend basicblock referenced by phi node to exist");

                    let mut backend_block = *backend_block;

                    let mut value = build_value(ctx, &value_catalog, &builder, &incoming.value)?;

                    LLVMAddIncoming(relocation.phi_node, &mut value, &mut backend_block, 1);
                }
            }
        }
    }

    Ok(())
}
