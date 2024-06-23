
#[derive(Clone, Debug)]
pub struct ExtendOptions {
    pub in_register: bool,
}

impl Default for ExtendOptions {
    fn default() -> Self {
        Self { in_register: false }
    }
}

