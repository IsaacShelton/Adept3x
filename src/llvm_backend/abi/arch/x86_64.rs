use super::super::abi_function::ABIFunction;

#[derive(Clone, Debug)]
pub struct X86_64 {
    pub is_windows: Variant,
}

#[derive(Clone, Debug)]
pub enum Variant {
    Normal,
    Win64,
}

impl X86_64 {
    pub fn compute_info(&self) -> ABIFunction {
        todo!("X86_64 function")
    }
}
