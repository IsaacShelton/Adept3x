use primitives::IntegerSign;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Syscall {
    Println,
    BuildAddProject,
    BuildSetAdeptVersion,
    BuildLinkFilename,
    BuildLinkFrameworkName,
    Experimental,
    ImportNamespace,
    DontAssumeIntAtLeast32Bits,
    UseDependency,
    Bake,
}

#[derive(Debug)]
pub enum ConstantValue {
    SmallData(u64),
}

impl ConstantValue {
    pub fn unwrap_small_data(&self) -> u64 {
        match self {
            ConstantValue::SmallData(value) => *value,
        }
    }
}

#[derive(Debug)]
pub enum ConstantValueSchema {
    Boolean,
    Integer(IntegerSign),
}
