use crate::{BasicBlockId, CfgBuilder, InstrRef};

#[derive(Clone, Debug)]
pub struct RevPostOrderIterWithEnds {
    current: Option<InstrRef>,
    post_order_index: u32,
}

impl RevPostOrderIterWithEnds {
    pub fn new(cfg: &CfgBuilder, post_order: &[BasicBlockId]) -> Self {
        if post_order.len() == 0 {
            return Self {
                current: None,
                post_order_index: 0,
            };
        }

        let mut post_order_index = post_order.len() - 1;

        loop {
            let bb_id = post_order[post_order_index];

            if cfg.get_unsafe(bb_id).inner_len() != 0 {
                return Self {
                    current: Some(InstrRef::new(bb_id, 0)),
                    post_order_index: post_order_index.try_into().unwrap(),
                };
            }

            if post_order_index == 0 {
                return Self {
                    current: None,
                    post_order_index: 0,
                };
            }

            post_order_index -= 1;
        }
    }

    pub fn peek(&self) -> Option<InstrRef> {
        self.current
    }

    pub fn next(&mut self, cfg: &CfgBuilder, post_order: &[BasicBlockId]) -> Option<InstrRef> {
        let current = self.current?;
        let bb = cfg.get_unsafe(current.basicblock);

        // NOTE: We don't subtract one, since we want to include the end instruction.
        if current.instr_or_end < bb.inner_len() {
            self.current = Some(InstrRef::new(current.basicblock, current.instr_or_end + 1));
            return self.current;
        }

        while self.post_order_index > 0 {
            self.post_order_index -= 1;
            let bb_id = post_order[self.post_order_index as usize];

            if cfg.get_unsafe(bb_id).inner_len() != 0 {
                self.current = Some(InstrRef::new(bb_id, 0));
                return self.current;
            }
        }

        self.current = None;
        return None;
    }
}
