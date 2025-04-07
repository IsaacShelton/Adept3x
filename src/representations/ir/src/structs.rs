use crate::Field;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: Option<String>,
    pub fields: Vec<Field>,
    pub is_packed: bool,
    pub source: Source,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructRef {
    index: usize,
}

impl StructRef {
    pub fn new(index: usize) -> Self {
        Self { index }
    }

    pub fn get(&self) -> usize {
        self.index
    }
}

#[derive(Debug, Default)]
pub struct Structs {
    structs: Box<[Struct]>,
}

impl Structs {
    pub fn new(structs: Box<[Struct]>) -> Self {
        Self { structs }
    }

    pub fn get(&self, key: StructRef) -> &Struct {
        &self.structs[key.index]
    }
}
