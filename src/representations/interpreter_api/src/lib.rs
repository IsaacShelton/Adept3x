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

#[derive(Debug)]
pub enum ConstantValueSchema {
    Boolean,
    Integer(IntegerSign),
}
