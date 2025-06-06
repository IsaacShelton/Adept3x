use super::{
    attribute::{add_func_attribute, create_enum_attribute},
    block::create_function_block,
    epilogue::{EpilogueInfo, emit_epilogue},
    prologue::{PrologueInfo, emit_prologue},
};
use crate::{builder::Builder, ctx::BackendCtx, value_catalog::ValueCatalog, values::build_value};
use backend::BackendError;
use cstr::cstr;
use data_units::AtomicByteUnits;
use llvm_sys::{
    core::{LLVMAddIncoming, LLVMAppendBasicBlock, LLVMGetUndef, LLVMInt32Type},
    prelude::{LLVMBasicBlockRef, LLVMValueRef},
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
    pub alloca_point: Option<LLVMValueRef>,
    pub max_vector_width_bytes: AtomicByteUnits,
}

pub unsafe fn create_function_bodies(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (ir_func_ref, skeleton) in ctx.func_skeletons.iter() {
        let ir_function = &ctx.ir_module.funcs[*ir_func_ref];

        let mut builder = Builder::new();
        let mut value_catalog = ValueCatalog::new(ir_function.basicblocks.len());

        let entry_basicblock = (!ir_function.basicblocks.is_empty())
            .then(|| LLVMAppendBasicBlock(skeleton.function, cstr!("prologue").as_ptr()));

        let alloca_point = entry_basicblock.map(|entry_basicblock| {
            let undef = unsafe { LLVMGetUndef(LLVMInt32Type()) };
            builder.position(entry_basicblock);
            builder.bitcast_with_name(undef, unsafe { LLVMInt32Type() }, cstr!("allocaapt"))
        });

        let prologue = match skeleton.abi_function.as_ref().zip(alloca_point) {
            Some((abi_function, alloca_point)) => {
                let prologue_block = entry_basicblock.unwrap();

                Some(emit_prologue(
                    ctx,
                    &builder,
                    skeleton,
                    abi_function,
                    alloca_point,
                    prologue_block,
                )?)
            }
            _ => None,
        };

        let epilogue = prologue
            .as_ref()
            .map(|prologue| {
                let epilogue_block =
                    LLVMAppendBasicBlock(skeleton.function, cstr!("epilogue").as_ptr());

                emit_epilogue(
                    ctx,
                    &builder,
                    skeleton,
                    epilogue_block,
                    prologue.return_location.as_ref(),
                    prologue.alloca_point,
                )
            })
            .transpose()?;

        let fn_ctx = FnCtx {
            prologue,
            epilogue,
            overflow_basicblock: OnceCell::new(),
            alloca_point,
            max_vector_width_bytes: AtomicByteUnits::of(0),
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
        } else if let Some(entry_basicblock) = entry_basicblock {
            builder.position(entry_basicblock);
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

        let max_vector_width = fn_ctx.max_vector_width_bytes.into_inner();

        if !max_vector_width.is_zero() && ctx.arch.is_x_86_64() {
            let nounwind = create_enum_attribute(
                cstr!("min-legal-vector-width"),
                max_vector_width.to_bits().bits(),
            );
            add_func_attribute(skeleton.function, nounwind);
        }
    }

    Ok(())
}
