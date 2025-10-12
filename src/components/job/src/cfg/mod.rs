mod builder;
mod flatten;
mod instr;
mod value;

use crate::{BuiltinTypes, repr::UnaliasedType};
use arena::{Arena, Id, Idx, new_id_with_niche};
pub use builder::*;
use diagnostics::ErrorDiagnostic;
pub use flatten::*;
pub use instr::*;
use source_files::Source;
use std::{collections::HashMap, fmt::Display};
pub use value::*;

new_id_with_niche!(BasicBlockId, u32);

#[derive(Copy, Clone, Debug)]
pub enum IsValue {
    RequireValue,
    NeglectValue,
}

impl Display for BasicBlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.0.get())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstrRef {
    pub basicblock: BasicBlockId,
    pub instr_or_end: u32,
}

impl Display for InstrRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.basicblock, self.instr_or_end)
    }
}

impl InstrRef {
    pub fn new(basicblock: BasicBlockId, instr_or_end: u32) -> Self {
        Self {
            basicblock,
            instr_or_end,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Label<'env> {
    pub name: &'env str,
    pub target: BasicBlockId,
    pub source: Source,
}

impl<'env> Label<'env> {
    pub fn new(name: &'env str, target: BasicBlockId, source: Source) -> Self {
        Self {
            name,
            target,
            source,
        }
    }
}

#[derive(Debug)]
pub struct Cfg<'env> {
    pub basicblocks: Arena<BasicBlockId, BasicBlock<'env>>,
    pub labels: &'env [Label<'env>],
}

impl<'env> Cfg<'env> {
    pub fn new() -> Self {
        Self {
            basicblocks: Arena::new(),
            labels: &[],
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
    pub fn get_unsafe(&self, id: BasicBlockId) -> &BasicBlock<'env> {
        &self.basicblocks[unsafe { Idx::from_raw(id) }]
    }

    #[inline]
    pub fn get_end_ref(&self, id: BasicBlockId) -> InstrRef {
        let bb = self.get_unsafe(id);
        InstrRef::new(id, bb.instrs.len().try_into().unwrap())
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

    pub fn get_typed(
        &self,
        cfg_value: CfgValue,
        builtin_types: &'env BuiltinTypes<'env>,
    ) -> UnaliasedType<'env> {
        let CfgValue::Instr(instr_ref) = cfg_value else {
            return builtin_types.void();
        };

        let bb = &self.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];
        assert!((instr_ref.instr_or_end as usize) < bb.instrs.len());
        bb.instrs[instr_ref.instr_or_end as usize].typed.unwrap()
    }
}

impl<'env> Display for Cfg<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CFG WITH {} BASICBLOCKS", self.len())?;

        for (i, block) in &self.basicblocks {
            let i = i.into_raw();
            writeln!(f, "{}\n{}", i, block)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct BasicBlock<'env> {
    pub instrs: &'env [Instr<'env>],
    pub end: EndInstr<'env>,
}

impl<'env> Display for BasicBlock<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, instr) in self.instrs.iter().enumerate() {
            write!(f, "  {:04} {}", index, instr)?;
        }
        write!(f, "  {:04} {}", self.instrs.len(), self.end)?;
        Ok(())
    }
}

impl<'env> Display for PartialBasicBlock<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, instr) in self.instrs.iter().enumerate() {
            write!(f, "  {:04} {}", index, instr)?;
        }
        if let Some(end) = &self.end {
            write!(f, "  {:04} {}", self.instrs.len(), end)?;
        } else {
            writeln!(f, "  {:04} <missing>", self.instrs.len())?;
        }
        Ok(())
    }
}
