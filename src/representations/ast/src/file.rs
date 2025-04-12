use super::{
    ExprAlias, Func, Trait, enumeration::Enum, global_variable::Global, implementation::Impl,
    structs::Struct, type_alias::TypeAlias,
};

#[derive(Clone, Debug)]
pub struct RawAstFile {
    pub funcs: Vec<Func>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub globals: Vec<Global>,
    pub type_aliases: Vec<TypeAlias>,
    pub expr_aliases: Vec<ExprAlias>,
    pub traits: Vec<Trait>,
    pub impls: Vec<Impl>,
}

impl RawAstFile {
    pub fn new() -> RawAstFile {
        RawAstFile {
            funcs: vec![],
            structs: vec![],
            enums: vec![],
            globals: vec![],
            type_aliases: vec![],
            expr_aliases: vec![],
            traits: vec![],
            impls: vec![],
        }
    }
}
