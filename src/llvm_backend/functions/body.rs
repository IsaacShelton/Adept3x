use super::epilogue::{emit_epilogue, EpilogueInfo};
use super::prologue::emit_prologue;
use super::{block::create_function_block, prologue::PrologueInfo};
use crate::{
    ir,
    llvm_backend::{
        builder::Builder, ctx::BackendCtx, error::BackendError, value_catalog::ValueCatalog,
        values::build_value,
    },
};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMAddIncoming, LLVMAppendBasicBlock},
    prelude::LLVMBasicBlockRef,
};
use std::cell::OnceCell;

pub struct BasicBlockInfo<'a> {
    pub id: usize,
    pub ir_basicblock: &'a ir::BasicBlock,
    pub llvm_basicblock: LLVMBasicBlockRef,
}

pub struct FnCtx {
    pub prologue: Option<PrologueInfo>,
    pub epilogue: Option<EpilogueInfo>,
    pub overflow_basicblock: OnceCell<LLVMBasicBlockRef>,
}

pub unsafe fn create_function_bodies(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (ir_function_ref, skeleton) in ctx.func_skeletons.iter() {
        if let Some(ir_function) = ctx.ir_module.functions.get(ir_function_ref) {
            let mut builder = Builder::new();
            let mut value_catalog = ValueCatalog::new(ir_function.basicblocks.len());

            let prologue = match skeleton.abi_function.as_ref() {
                Some(abi_function) if !ir_function.basicblocks.is_empty() => {
                    let prologue_block =
                        LLVMAppendBasicBlock(skeleton.function, cstr!("prologue").as_ptr());

                    Some(emit_prologue(
                        ctx,
                        &builder,
                        skeleton,
                        abi_function,
                        prologue_block,
                    )?)
                }
                _ => None,
            };

            let epilogue = prologue.as_ref().map(|prologue| {
                let epilogue_block =
                    LLVMAppendBasicBlock(skeleton.function, cstr!("epilogue").as_ptr());

                emit_epilogue(&builder, epilogue_block)
            });

            let fn_ctx = FnCtx {
                prologue,
                epilogue,
                overflow_basicblock: OnceCell::new(),
            };

            let basicblocks = ir_function
                .basicblocks
                .iter()
                .enumerate()
                .map(|(id, ir_basicblock)| BasicBlockInfo {
                    id,
                    ir_basicblock,
                    llvm_basicblock: LLVMAppendBasicBlock(skeleton.function, cstr!("").as_ptr()),
                })
                .collect::<Vec<_>>();

            // Jump to first basicblock after prologue
            if let Some(prologue) = fn_ctx.prologue.as_ref() {
                builder.position(prologue.last_llvm_block);
                builder.br(basicblocks
                    .first()
                    .expect("function has body")
                    .llvm_basicblock);
            }

            for basicblock in basicblocks.iter() {
                create_function_block(
                    ctx,
                    &builder,
                    basicblock,
                    skeleton.function,
                    &basicblocks,
                    &fn_ctx,
                    &mut value_catalog,
                )?;
            }
            unsafe { llvm_sys::core::LLVMDumpModule(ctx.backend_module.get()) };

            for relocation in builder.take_phi_relocations().iter() {
                for incoming in relocation.incoming.iter() {
                    let mut backend_block = basicblocks
                        .get(incoming.basicblock_id)
                        .expect("backend basicblock referenced by phi node to exist")
                        .llvm_basicblock;

                    let mut value = build_value(ctx, &value_catalog, &builder, &incoming.value)?;
                    LLVMAddIncoming(relocation.phi_node, &mut value, &mut backend_block, 1);
                }
            }
        }
    }

    Ok(())
}
