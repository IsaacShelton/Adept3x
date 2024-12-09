use crate::target::Target;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub emit_llvm_ir: bool,
    pub emit_ir: bool,
    pub interpret: bool,
    pub coerce_main_signature: bool,
    pub excute_result: bool,
    pub allow_experimental_pragma_features: bool,
    pub use_pic: Option<bool>,
    pub target: Target,
    pub infrastructure: Option<PathBuf>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        let current_exe = std::env::current_exe()
            .expect("failed to get adept executable location")
            .parent()
            .expect("parent folder")
            .to_path_buf();

        let infrastructure = current_exe.join("infrastructure");

        Self {
            emit_llvm_ir: false,
            emit_ir: false,
            interpret: false,
            coerce_main_signature: true,
            excute_result: false,
            allow_experimental_pragma_features: false,
            use_pic: None,
            target: Target::HOST,
            infrastructure: Some(infrastructure),
        }
    }
}
