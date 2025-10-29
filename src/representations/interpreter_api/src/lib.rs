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

pub enum ConstantValue {
    Integer(u64),
}

pub enum ConstantValueSchema {
    Integer(IntegerSign),
}
