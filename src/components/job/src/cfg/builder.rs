pub use super::*;
use crate::{
    ExecutionCtx,
    cfg::instr::BreakContinue,
    repr::{Type, UnaliasedType},
};
use arena::Idx;

#[derive(Clone, Debug, Default)]
pub struct PartialBasicBlock<'env> {
    pub instrs: Vec<Instr<'env>>,
    pub end: Option<EndInstr<'env>>,
}

impl<'env> PartialBasicBlock<'env> {
    pub fn inner_len(&self) -> u32 {
        self.instrs
            .len()
            .try_into()
            .expect("cannot have more than 2^32-1 instructions in a basicblock")
    }
}

pub struct Cursor {
    basicblock: BasicBlockId,
}

impl Cursor {
    pub fn basicblock<'env>(&self) -> Idx<BasicBlockId, PartialBasicBlock<'env>> {
        // We will allow sharing indicies
        unsafe { Idx::from_raw(self.basicblock) }
    }
}

impl<'env> Cursor {
    pub fn new(basicblock: BasicBlockId) -> Self {
        Self { basicblock }
    }
}

#[derive(Clone, Debug)]
pub struct CfgBuilder<'env> {
    pub basicblocks: Arena<BasicBlockId, PartialBasicBlock<'env>>,
    pub labels: Vec<Label<'env>>,
}

impl<'env> CfgBuilder<'env> {
    pub fn new() -> (Self, Cursor) {
        let mut basicblocks = Arena::new();
        let cursor = Cursor::new(basicblocks.alloc(PartialBasicBlock::default()).into_raw());

        (
            Self {
                basicblocks,
                labels: Vec::new(),
            },
            cursor,
        )
    }

    pub fn set_typed(&mut self, instr_ref: InstrRef, ty: UnaliasedType<'env>) {
        let bb = &mut self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];
        assert!((instr_ref.instr_or_end as usize) < bb.instrs.len());
        bb.instrs[instr_ref.instr_or_end as usize].typed = Some(ty);
    }

    pub fn get_typed(&self, instr_ref: InstrRef) -> UnaliasedType<'env> {
        let bb = &self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];
        assert!((instr_ref.instr_or_end as usize) < bb.instrs.len());
        bb.instrs[instr_ref.instr_or_end as usize].typed.unwrap()
    }

    #[inline]
    pub fn start(&self) -> BasicBlockId {
        assert_ne!(self.basicblocks.len(), 0);
        BasicBlockId::from_usize(0)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.basicblocks.len()
    }

    #[inline]
    pub fn get_unsafe(&self, id: BasicBlockId) -> &PartialBasicBlock<'env> {
        &self.basicblocks[unsafe { Idx::from_raw(id) }]
    }

    pub fn iter_instrs_ordered(&self) -> impl Iterator<Item = (InstrRef, &Instr<'env>)> {
        self.basicblocks.iter().flat_map(|(bb_id, bb)| {
            bb.instrs.iter().enumerate().map(move |(i, instr)| {
                (
                    InstrRef::new(bb_id.into_raw(), i.try_into().unwrap()),
                    instr,
                )
            })
        })
    }

    pub fn add_label(&mut self, label: Label<'env>) {
        self.labels.push(label);
    }

    pub fn new_block(&mut self) -> Cursor {
        Cursor::new(
            self.basicblocks
                .alloc(PartialBasicBlock::default())
                .into_raw(),
        )
    }

    pub fn push_jump_to_new_block(&mut self, cursor: &mut Cursor, source: Source) {
        let new_block = self.new_block();

        self.push_end(
            cursor,
            EndInstrKind::Jump(new_block.basicblock().into_raw()).at(source),
        );

        *cursor = new_block;
    }

    pub fn finish(self, ctx: &mut ExecutionCtx<'env>) -> Cfg<'env> {
        let labels = ctx.alloc_slice_fill_iter(self.labels.into_iter());

        let mut basicblocks = Arena::with_capacity(self.basicblocks.len());

        for (_, pbb) in self.basicblocks {
            let instrs = ctx.alloc_slice_fill_iter(pbb.instrs.into_iter());

            basicblocks.alloc(BasicBlock {
                instrs,
                end: pbb.end.expect("all cfg basicblocks must have an end"),
            });
        }

        Cfg {
            basicblocks,
            labels,
        }
    }

    pub fn has_end(&self, cursor: &Cursor) -> bool {
        self.basicblocks[cursor.basicblock()].end.is_some()
    }

    pub fn push(&mut self, cursor: &mut Cursor, instr: Instr<'env>) -> InstrRef {
        let bb = &mut self.basicblocks[cursor.basicblock()];
        let new_instr = InstrRef::new(cursor.basicblock, bb.inner_len());

        if !bb.end.is_some() {
            bb.instrs.push(instr);
        }

        new_instr
    }

    pub fn push_end(&mut self, cursor: &mut Cursor, end_instr: EndInstr<'env>) -> InstrRef {
        let bb = &mut self.basicblocks[cursor.basicblock()];
        bb.end.get_or_insert(end_instr);
        InstrRef::new(cursor.basicblock, bb.inner_len())
    }

    pub fn push_branch(
        &mut self,
        condition: InstrRef,
        cursor: &mut Cursor,
        break_continue: Option<BreakContinue>,
        source: Source,
    ) -> (Cursor, Cursor) {
        let bb = &mut self.basicblocks[cursor.basicblock()];

        if bb.end.is_some() {
            return (
                Cursor::new(cursor.basicblock),
                Cursor::new(cursor.basicblock),
            );
        }

        let when_true = Cursor::new(
            self.basicblocks
                .alloc(PartialBasicBlock::default())
                .into_raw(),
        );
        let when_false = Cursor::new(
            self.basicblocks
                .alloc(PartialBasicBlock::default())
                .into_raw(),
        );

        self.push_end(
            cursor,
            EndInstrKind::Branch(
                condition,
                when_true.basicblock,
                when_false.basicblock,
                break_continue,
            )
            .at(source),
        );

        (when_true, when_false)
    }

    pub fn push_scope(&mut self, cursor: &mut Cursor, source: Source) -> (Cursor, Cursor) {
        let bb = &mut self.basicblocks[cursor.basicblock()];

        if bb.end.is_some() {
            return (
                Cursor::new(cursor.basicblock),
                Cursor::new(cursor.basicblock),
            );
        }

        let in_scope = Cursor::new(
            self.basicblocks
                .alloc(PartialBasicBlock::default())
                .into_raw(),
        );
        let close_scope = Cursor::new(
            self.basicblocks
                .alloc(PartialBasicBlock::default())
                .into_raw(),
        );

        self.push_end(
            cursor,
            EndInstrKind::NewScope(in_scope.basicblock, close_scope.basicblock).at(source),
        );

        (in_scope, close_scope)
    }

    pub fn finalize_gotos(mut self) -> Result<Self, ErrorDiagnostic> {
        // Create map of labels to target basicblocks
        let mut labels = HashMap::with_capacity(self.labels.len());
        for label in std::mem::take(&mut self.labels).into_iter() {
            if labels.contains_key(&label.name) {
                return Err(ErrorDiagnostic::new(
                    format!("Duplicate label '@{}@'", &label.name),
                    label.source,
                ));
            }

            assert_eq!(labels.insert(label.name, label.target), None);
        }

        // Replace incomplete gotos with direct jumps to the target basicblocks
        for bb in self.basicblocks.values_mut() {
            if let EndInstrKind::IncompleteGoto(label_name) = &mut bb.end.as_mut().unwrap().kind {
                let Some(target) = labels.get(label_name) else {
                    return Err(ErrorDiagnostic::new(
                        format!("Undefined label '@{}@'", label_name),
                        bb.end.as_mut().unwrap().source,
                    ));
                };

                bb.end.as_mut().unwrap().kind = EndInstrKind::Jump(*target);
            }
        }

        Ok(self)
    }
}

impl<'env> Display for CfgBuilder<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BUILDER WITH {} BASICBLOCKS", self.basicblocks.len())?;

        for (i, block) in &self.basicblocks {
            let i = i.into_raw();
            writeln!(f, "{}\n{}", i, block)?;
        }
        Ok(())
    }
}
