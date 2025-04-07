use crate::{BasicBlocks, Type};
use attributes::SymbolOwnership;

#[derive(Clone, Debug)]
pub struct Func {
    pub mangled_name: String,
    pub params: Vec<Type>,
    pub return_type: Type,
    pub basicblocks: BasicBlocks,
    pub is_cstyle_variadic: bool,
    pub ownership: SymbolOwnership,
    pub abide_abi: bool,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct FuncRef {
    index: usize,
}

impl FuncRef {
    pub fn new(index: usize) -> Self {
        Self { index }
    }

    pub fn get(&self) -> usize {
        self.index
    }
}

#[derive(Default)]
pub struct Funcs {
    funcs: Box<[Func]>,
}

impl Funcs {
    pub fn new(funcs: Box<[Func]>) -> Self {
        Self { funcs }
    }

    pub fn get(&self, key: FuncRef) -> &Func {
        &self.funcs[key.index]
    }

    pub fn get_mut(&mut self, key: FuncRef) -> &mut Func {
        &mut self.funcs[key.index]
    }

    pub fn values(&self) -> impl Iterator<Item = &Func> {
        self.funcs.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (FuncRef, &Func)> {
        self.funcs
            .iter()
            .enumerate()
            .map(|(index, function)| (FuncRef { index }, function))
    }
}
