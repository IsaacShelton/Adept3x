use crate::{BasicBlockId, CfgBuilder, InstrRef};

#[derive(Clone, Debug, Default)]
pub struct PostOrderIterWithEnds {
    post_order_index: u32,
    instr_index: Option<u32>,
}

impl PostOrderIterWithEnds {
    pub fn next<'env>(
        &mut self,
        cfg: &CfgBuilder<'env>,
        post_order: &[BasicBlockId],
    ) -> Option<InstrRef> {
        loop {
            if self.post_order_index as usize >= post_order.len() {
                return None;
            }

            let bb_id = post_order[self.post_order_index as usize];
            let bb = cfg.get_unsafe(bb_id);

            if bb.instrs.len() == 0 {
                self.post_order_index += 1;
                self.instr_index = None;
                continue;
            }

            let instr_index = *self
                .instr_index
                .get_or_insert(bb.instrs.len().try_into().unwrap());
            let instr_ref = InstrRef::new(bb_id, instr_index);

            if instr_index == 0 {
                self.instr_index = None;
                self.post_order_index += 1;
            } else {
                self.instr_index = Some(instr_index - 1);
            }

            return Some(instr_ref);
        }
    }
}
