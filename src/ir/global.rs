use super::Type;
use crate::ast::Exposure;

#[derive(Clone, Debug)]
pub struct Global {
    pub mangled_name: String,
    pub ir_type: Type,
    pub is_foreign: bool,
    pub is_thread_local: bool,
    pub exposure: Exposure,
}
