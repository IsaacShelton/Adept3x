pub use super::*;
use crate::{
    BuiltinTypes, ExecutionCtx,
    cfg::instr::{BreakContinue, CallTarget},
    conform::UnaryCast,
    module_graph::ModuleView,
    repr::{FuncHead, VariableRef},
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

    pub fn display<'a, 'b, 'c>(
        &'a self,
        view: &'b ModuleView<'env>,
        disambiguation: &'c TypeDisplayerDisambiguation<'env>,
    ) -> PartialBasicBlockDisplayer<'a, 'b, 'c, 'env> {
        PartialBasicBlockDisplayer {
            basicblock: self,
            view,
            disambiguation,
        }
    }
}

pub struct PartialBasicBlockDisplayer<'a, 'b, 'c, 'env: 'a + 'b + 'c> {
    basicblock: &'a PartialBasicBlock<'env>,
    view: &'b ModuleView<'env>,
    disambiguation: &'c TypeDisplayerDisambiguation<'env>,
}

impl<'a, 'b, 'c, 'env: 'a + 'b + 'c> Display for PartialBasicBlockDisplayer<'a, 'b, 'c, 'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, instr) in self.basicblock.instrs.iter().enumerate() {
            write!(
                f,
                "  {:04} {}",
                index,
                instr.display(self.view, self.disambiguation)
            )?;
        }
        if let Some(end) = &self.basicblock.end {
            write!(f, "  {:04} {}", self.basicblock.instrs.len(), end)?;
        } else {
            writeln!(f, "  {:04} <missing>", self.basicblock.instrs.len())?;
        }
        Ok(())
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

    pub fn never_or_void(&self, cursor: &Cursor) -> CfgValue {
        let bb = &self.basicblocks[cursor.basicblock()];

        if bb.end.is_some() {
            CfgValue::Instr(InstrRef::new(
                cursor.basicblock().into_raw(),
                bb.instrs.len().try_into().unwrap(),
            ))
        } else {
            CfgValue::Void
        }
    }

    pub fn set_pre_jump_typed_unary_cast(
        &mut self,
        bb_id: BasicBlockId,
        cast: Option<UnaryCast<'env>>,
        to_ty: UnaliasedType<'env>,
    ) {
        let bb = &mut self.basicblocks[unsafe { Idx::from_raw(bb_id) }];
        match &mut bb.end.as_mut().unwrap().kind {
            EndInstrKind::Jump(_, _, unary_cast, ty) => {
                *unary_cast = cast;
                *ty = Some(to_ty);
            }
            _ => panic!("cannot set_jump_pre_conform for non-jump"),
        }
    }

    pub fn set_typed(&mut self, instr_ref: InstrRef, typed: UnaliasedType<'env>) {
        let bb = &mut self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];
        assert!((instr_ref.instr_or_end as usize) < bb.instrs.len());
        let instr = &mut bb.instrs[instr_ref.instr_or_end as usize];
        instr.typed = Some(typed);
    }

    pub fn set_typed_and_callee(
        &mut self,
        instr_ref: InstrRef,
        callee: &'env FuncHead<'env>,
        arg_casts: &'env [Option<UnaryCast<'env>>],
        variadic_arg_types: &'env [UnaliasedType<'env>],
        view: &'env ModuleView<'env>,
    ) {
        let bb = &mut self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];
        assert!((instr_ref.instr_or_end as usize) < bb.instrs.len());
        let instr = &mut bb.instrs[instr_ref.instr_or_end as usize];

        match &mut instr.kind {
            InstrKind::Call(_, target) => {
                *target = Some(CallTarget {
                    callee,
                    arg_casts,
                    variadic_arg_types,
                    view,
                });
            }
            _ => panic!("cannot set_typed_and_callee for non-call"),
        }

        instr.typed = Some(callee.return_type);
    }

    pub fn set_struct_literal_unary_casts_and_indices(
        &mut self,
        instr_ref: InstrRef,
        unary_casts: &'env [(usize, Option<UnaryCast<'env>>)],
    ) {
        let bb = &mut self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];
        assert!((instr_ref.instr_or_end as usize) < bb.instrs.len());
        let instr = &mut bb.instrs[instr_ref.instr_or_end as usize];

        match &mut instr.kind {
            InstrKind::StructLiteral(_, field_value_casts) => {
                *field_value_casts = Some(unary_casts);
            }
            _ => panic!("cannot set_struct_literal_unary_casts for non-struct-literal"),
        }
    }

    pub fn get_typed(
        &self,
        cfg_value: CfgValue,
        builtin_types: &'env BuiltinTypes<'env>,
    ) -> UnaliasedType<'env> {
        let CfgValue::Instr(instr_ref) = cfg_value else {
            return builtin_types.void();
        };

        let bb = &self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];

        if (instr_ref.instr_or_end as usize) < bb.instrs.len() {
            bb.instrs[instr_ref.instr_or_end as usize].typed.unwrap()
        } else {
            let end = bb.end.as_ref().unwrap();

            match end.kind {
                EndInstrKind::Jump(_, _, _, unaliased_type) => unaliased_type.unwrap(),
                // If nothing inside of taken branch, result is void
                EndInstrKind::Branch(_, _, _, _) | EndInstrKind::NewScope(_, _) => {
                    builtin_types.void()
                }
                _ => panic!("cannot get type of non-jump end instruction"),
            }
        }
    }

    pub fn set_primary_unary_cast(&mut self, instr_ref: InstrRef, cast: Option<UnaryCast<'env>>) {
        let bb = &mut self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];

        // End Instruction
        if instr_ref.instr_or_end as usize >= bb.instrs.len() {
            match &mut bb.end.as_mut().unwrap().kind {
                EndInstrKind::Return(_, unary_cast) => *unary_cast = cast,
                _ => unreachable!(),
            }
            return;
        }

        // Sequential Instruction
        match &mut bb.instrs[instr_ref.instr_or_end as usize].kind {
            InstrKind::DeclareAssign(_, _, unary_cast, _)
            | InstrKind::Declare(_, _, _, unary_cast, _)
            | InstrKind::ConformToBool(_, _, unary_cast)
            | InstrKind::Assign {
                dest: _,
                src: _,
                src_cast: unary_cast,
            }
            | InstrKind::UnaryOperation(_, _, unary_cast)
            | InstrKind::IntoDest(_, unary_cast) => *unary_cast = cast,
            _ => unreachable!(),
        }
    }

    pub fn set_binop_unary_casts(
        &mut self,
        instr_ref: InstrRef,
        a_cast: Option<UnaryCast<'env>>,
        b_cast: Option<UnaryCast<'env>>,
    ) {
        let bb = &mut self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];

        // Sequential Instruction
        match &mut bb.instrs[instr_ref.instr_or_end as usize].kind {
            InstrKind::BinOp(_, _, _, _, a_unary_cast, b_unary_cast, _) => {
                *a_unary_cast = a_cast;
                *b_unary_cast = b_cast;
            }
            _ => unreachable!(),
        }
    }

    pub fn set_variable_ref(&mut self, instr_ref: InstrRef, new_variable_ref: VariableRef<'env>) {
        let bb = &mut self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];

        // Sequential Instruction
        match &mut bb.instrs[instr_ref.instr_or_end as usize].kind {
            InstrKind::DeclareAssign(_, _, _, variable_ref)
            | InstrKind::Declare(_, _, _, _, variable_ref)
            | InstrKind::Name(_, variable_ref)
            | InstrKind::Parameter(_, _, _, variable_ref) => *variable_ref = Some(new_variable_ref),
            _ => unreachable!(),
        }
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

        self.try_push_end(
            EndInstrKind::Jump(
                new_block.basicblock().into_raw(),
                self.never_or_void(cursor),
                None,
                None,
            )
            .at(source),
            cursor,
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

    pub fn try_push(&mut self, cursor: &mut Cursor, instr: Instr<'env>) -> InstrRef {
        let bb = &mut self.basicblocks[cursor.basicblock()];
        let new_instr = InstrRef::new(cursor.basicblock, bb.inner_len());

        if !bb.end.is_some() {
            bb.instrs.push(instr);
        }

        new_instr
    }

    pub fn try_push_end(&mut self, end_instr: EndInstr<'env>, cursor: &mut Cursor) -> InstrRef {
        let bb = &mut self.basicblocks[cursor.basicblock()];
        bb.end.get_or_insert(end_instr);
        InstrRef::new(cursor.basicblock, bb.inner_len())
    }

    pub fn try_push_branch(
        &mut self,
        condition: CfgValue,
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

        self.try_push_end(
            EndInstrKind::Branch(
                condition,
                when_true.basicblock,
                when_false.basicblock,
                break_continue,
            )
            .at(source),
            cursor,
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

        self.try_push_end(
            EndInstrKind::NewScope(in_scope.basicblock, close_scope.basicblock).at(source),
            cursor,
        );

        (in_scope, close_scope)
    }

    pub fn finalize_gotos(mut self) -> Result<Self, ErrorDiagnostic> {
        // Create map of labels to target basicblocks
        let mut labels = HashMap::with_capacity(self.labels.len());
        for label in std::mem::take(&mut self.labels).into_iter() {
            if labels.contains_key(&label.name) {
                return Err(ErrorDiagnostic::new(
                    format!("Duplicate label `@{}@`", &label.name),
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
                        format!("Undefined label `@{}@`", label_name),
                        bb.end.as_mut().unwrap().source,
                    ));
                };

                bb.end.as_mut().unwrap().kind =
                    EndInstrKind::Jump(*target, CfgValue::Void, None, None);
            }
        }

        Ok(self)
    }

    pub fn display<'a, 'b, 'c>(
        &'a self,
        view: &'b ModuleView<'env>,
        disambiguation: &'c TypeDisplayerDisambiguation<'env>,
    ) -> CfgBuilderDisplayer<'a, 'b, 'c, 'env> {
        CfgBuilderDisplayer {
            cfg_builder: self,
            view,
            disambiguation,
        }
    }
}

pub struct CfgBuilderDisplayer<'a, 'b, 'c, 'env: 'a + 'b + 'c> {
    cfg_builder: &'a CfgBuilder<'env>,
    view: &'b ModuleView<'env>,
    disambiguation: &'c TypeDisplayerDisambiguation<'env>,
}

impl<'a, 'b, 'c, 'env: 'a + 'b + 'c> Display for CfgBuilderDisplayer<'a, 'b, 'c, 'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "BUILDER WITH {} BASICBLOCKS",
            self.cfg_builder.basicblocks.len()
        )?;

        for (i, block) in &self.cfg_builder.basicblocks {
            let i = i.into_raw();
            writeln!(
                f,
                "{}\n{}",
                i,
                block.display(self.view, self.disambiguation)
            )?;
        }
        Ok(())
    }
}
