use super::{datatype::ConcreteType, error::LowerError};
use crate::{
    asg,
    ir::{self, BasicBlock, BasicBlocks, Instr, ValueReference},
    resolve::PolyRecipe,
};
use std::borrow::Cow;

pub struct Builder<'a> {
    basicblocks: BasicBlocks,
    current_basicblock_id: usize,
    poly_recipe: &'a PolyRecipe,
    pub break_basicblock_id: Option<usize>,
    pub continue_basicblock_id: Option<usize>,
}

impl<'a> Builder<'a> {
    pub fn new_with_starting_block(poly_recipe: &'a PolyRecipe) -> Self {
        let mut basicblocks = BasicBlocks::new();
        basicblocks.push(BasicBlock::new());

        Self {
            basicblocks,
            current_basicblock_id: 0,
            poly_recipe,
            break_basicblock_id: None,
            continue_basicblock_id: None,
        }
    }

    pub fn build(self) -> BasicBlocks {
        self.basicblocks
    }

    pub fn is_block_terminated(&self) -> bool {
        self.basicblocks.len() > 0
            && self.basicblocks.blocks[self.current_basicblock_id].is_terminated()
    }

    pub fn continues_to(&mut self, basicblock_id: usize) {
        if !self.is_block_terminated() {
            self.push(ir::Instr::Break(ir::Break { basicblock_id }));
        }
    }

    pub fn terminate(&mut self) {
        if !self.is_block_terminated() {
            self.push(Instr::Return(None));
        }
    }

    pub fn new_block(&mut self) -> usize {
        let block = BasicBlock::new();
        let id = self.basicblocks.len();
        self.basicblocks.push(block);
        id
    }

    pub fn use_block(&mut self, id: usize) {
        if id >= self.basicblocks.len() {
            panic!("attempt to build with basicblock that doesn't exist");
        }

        self.current_basicblock_id = id;
    }

    pub fn current_block_id(&mut self) -> usize {
        if self.basicblocks.len() == 0 {
            self.basicblocks.push(BasicBlock::new());
            0
        } else {
            self.current_basicblock_id
        }
    }

    pub fn push(&mut self, instruction: Instr) -> ir::Value {
        let current_block = self
            .basicblocks
            .get_mut(self.current_basicblock_id)
            .expect("at least one basicblock");

        current_block.push(instruction);

        ir::Value::Reference(ValueReference {
            basicblock_id: self.current_basicblock_id,
            instruction_id: current_block.instructions.len() - 1,
        })
    }

    pub fn unpoly(&self, ty: &asg::Type) -> Result<ConcreteType, LowerError> {
        self.poly_recipe
            .resolve_type(ty)
            .map(|x| ConcreteType(Cow::Owned(x)))
            .map_err(LowerError::from)
    }

    pub fn poly_recipe(&self) -> &'a PolyRecipe {
        self.poly_recipe
    }
}

pub fn unpoly<'a>(
    poly_recipe: &PolyRecipe,
    ty: &'a asg::Type,
) -> Result<ConcreteType<'a>, LowerError> {
    poly_recipe
        .resolve_type(ty)
        .map(|x| ConcreteType(Cow::Owned(x)))
        .map_err(LowerError::from)
}
