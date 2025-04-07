use super::Type;
use attributes::SymbolOwnership;

#[derive(Clone, Debug)]
pub struct Global {
    pub mangled_name: String,
    pub ir_type: Type,
    pub is_thread_local: bool,
    pub ownership: SymbolOwnership,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlobalRef {
    index: usize,
}

impl GlobalRef {
    pub fn new(index: usize) -> Self {
        Self { index }
    }

    pub fn get(&self) -> usize {
        self.index
    }
}

#[derive(Default)]
pub struct Globals {
    globals: Box<[Global]>,
}

impl Globals {
    pub fn new(globals: Box<[Global]>) -> Self {
        Self { globals }
    }

    pub fn get(&self, key: GlobalRef) -> &Global {
        &self.globals[key.index]
    }

    pub fn iter(&self) -> impl Iterator<Item = (GlobalRef, &Global)> {
        self.globals
            .iter()
            .enumerate()
            .map(|(index, global)| (GlobalRef::new(index), global))
    }
}
