use super::{datatype::ConcreteType, error::LowerError};
use crate::{
    ModBuilder,
    datatype::lower_type,
    expr::{lower_destination, lower_expr},
    stmts::lower_stmts,
};
use asg::{Asg, IntoPolyRecipeResolver, PolyRecipe};
use ir::{BasicBlock, BasicBlocks, Instr, ValueReference};
use std::borrow::Cow;
use target::Target;

pub struct FuncBuilder<'mod_builder, 'poly_recipe, 'asg, 'func> {
    basicblocks: BasicBlocks,
    current_basicblock_id: usize,
    mod_builder: &'mod_builder ModBuilder<'asg>,
    poly_recipe: &'poly_recipe PolyRecipe,
    asg_func: &'func asg::Func,
    pub break_basicblock_id: Option<usize>,
    pub continue_basicblock_id: Option<usize>,
}

impl<'mod_builder, 'poly_recipe, 'asg, 'func> FuncBuilder<'mod_builder, 'poly_recipe, 'asg, 'func> {
    pub fn new_with_starting_block(
        mod_builder: &'mod_builder ModBuilder<'asg>,
        poly_recipe: &'poly_recipe PolyRecipe,
        asg_func: &'func asg::Func,
    ) -> Self {
        let mut basicblocks = BasicBlocks::new();
        basicblocks.push(BasicBlock::new());

        Self {
            basicblocks,
            current_basicblock_id: 0,
            mod_builder,
            poly_recipe,
            asg_func,
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
            .resolver()
            .resolve_type(ty)
            .map(|x| ConcreteType(Cow::Owned(x)))
            .map_err(LowerError::from)
    }

    pub fn lower_type(&self, ty: &asg::Type) -> Result<ir::Type, LowerError> {
        lower_type(self.mod_builder(), &self.unpoly(ty)?)
    }

    pub fn lower_expr(&mut self, expr: &asg::Expr) -> Result<ir::Value, LowerError> {
        lower_expr(self, expr)
    }

    pub fn lower_destination(&mut self, dest: &asg::Destination) -> Result<ir::Value, LowerError> {
        lower_destination(self, dest)
    }

    pub fn lower_stmts(&mut self, stmts: &[asg::Stmt]) -> Result<ir::Value, LowerError> {
        lower_stmts(self, stmts)
    }

    pub fn poly_recipe(&self) -> &'poly_recipe PolyRecipe {
        self.poly_recipe
    }

    pub fn asg(&self) -> &'asg Asg {
        self.mod_builder.asg
    }

    pub fn target(&self) -> &'mod_builder Target {
        &self.mod_builder.target
    }

    pub fn mod_builder(&self) -> &'mod_builder ModBuilder<'mod_builder> {
        self.mod_builder
    }

    pub fn asg_func(&self) -> &'func asg::Func {
        self.asg_func
    }
}

pub fn unpoly<'a>(
    poly_recipe: &PolyRecipe,
    ty: &'a asg::Type,
) -> Result<ConcreteType<'a>, LowerError> {
    poly_recipe
        .resolver()
        .resolve_type(ty)
        .map(|x| ConcreteType(Cow::Owned(x)))
        .map_err(LowerError::from)
}
