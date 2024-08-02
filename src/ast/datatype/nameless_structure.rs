use crate::ast::Field;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AnonymousStruct {
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
}
