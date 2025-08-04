use derive_more::IsVariant;
use std::{num::NonZero, path::PathBuf};
use target::Target;

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub emit_llvm_ir: bool,
    pub emit_ir: bool,
    pub interpret: bool,
    pub coerce_main_signature: bool,
    pub execute_result: bool,
    pub allow_experimental_pragma_features: bool,
    pub use_pic: Option<bool>,
    pub target: Target,
    pub infrastructure: Option<PathBuf>,
    pub available_parallelism: NonZero<usize>,
    pub new_compilation_system: NewCompilationSystem,
}

// Gradual adoption of new compilation system.
// This will be removed once the transition is complete.
#[derive(Copy, Clone, Debug, IsVariant)]
pub enum NewCompilationSystem {
    // Old compilation system (will be removed once transition is complete)
    Legacy,
    // Fully use the new compilation system (this alters lexing/parsing as well)
    Full,
}

impl Default for BuildOptions {
    fn default() -> Self {
        let current_exe = std::env::current_exe()
            .expect("failed to get adept executable location")
            .parent()
            .expect("parent folder")
            .to_path_buf();

        let infrastructure = current_exe.join("infrastructure");
        let available_parallelism = NonZero::new(num_cpus::get()).unwrap();

        Self {
            emit_llvm_ir: false,
            emit_ir: false,
            interpret: false,
            coerce_main_signature: true,
            execute_result: false,
            allow_experimental_pragma_features: false,
            use_pic: None,
            target: Target::HOST,
            infrastructure: Some(infrastructure),
            available_parallelism,
            new_compilation_system: NewCompilationSystem::Legacy,
        }
    }
}
