#[derive(Clone, Debug)]
pub struct IndirectOptions {
    pub in_register: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for IndirectOptions {
    fn default() -> Self {
        Self { in_register: false }
    }
}
