use crate::{CfgValue, InstrRef, ir, repr::FuncBody};
use arena::Id;

#[derive(Clone, Debug)]
pub struct IrBuilder<'env> {
    basicblocks: Vec<Vec<ir::Instr<'env>>>,
    outputs: Vec<Vec<Option<ir::Value<'env>>>>,
    current_bb_index: Option<usize>,
    current_cfg_instr_index: usize,
}

impl<'env> IrBuilder<'env> {
    pub fn new(body: &FuncBody<'env>) -> Self {
        let outputs = Vec::from_iter(
            body.cfg
                .basicblocks
                .values()
                .map(|bb| Vec::from_iter(std::iter::repeat_n(None, bb.instrs.len() + 1))),
        );

        let basicblocks = Vec::from_iter(body.cfg.basicblocks.values().map(|_| Vec::new()));

        Self {
            basicblocks,
            outputs,
            current_bb_index: None,
            current_cfg_instr_index: 0,
        }
    }

    pub fn set_position(&mut self, new_bb_index: usize) {
        if self.current_bb_index != Some(new_bb_index) {
            self.current_bb_index = Some(new_bb_index);
            self.current_cfg_instr_index = 0;
        }
    }

    pub fn push(&mut self, instr: ir::Instr<'env>) -> ir::Value<'env> {
        let current_bb_index = self.current_bb_index.unwrap();
        let current_block = &mut self.basicblocks[current_bb_index];
        current_block.push(instr);

        ir::Value::Reference(ir::ValueReference {
            basicblock_id: current_bb_index,
            instruction_id: current_block.len() - 1,
        })
    }

    pub fn push_output(&mut self, value: ir::Value<'env>) {
        self.outputs[self.current_bb_index.unwrap()][self.current_cfg_instr_index] = Some(value);
        self.current_cfg_instr_index += 1;
    }

    pub fn get_output(&self, cfg_value: CfgValue) -> ir::Value<'env> {
        let CfgValue::Instr(instr_ref) = cfg_value else {
            return ir::Literal::Void.into();
        };

        *self.outputs[instr_ref.basicblock.into_usize()][instr_ref.instr_or_end as usize]
            .as_ref()
            .unwrap()
    }

    pub fn finish(&mut self) -> Vec<Vec<ir::Instr<'env>>> {
        std::mem::take(&mut self.basicblocks)
    }
}
