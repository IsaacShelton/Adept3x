use crate::{
    BuiltinTypes,
    module_graph::{ComptimeKind, ModuleGraphRef},
};
use once_map::OnceMap;
use source_files::SourceFiles;
use std::path::Path;
use target::Target;

// This will be a more limited version of `compiler::Compiler`
// while we transition to the new job system, which we can then remove
// `compiler::Compiler` in favor of this...
pub struct Compiler<'env> {
    pub source_files: &'env SourceFiles,
    pub project_root: &'env Path,
    pub builtin_types: &'env BuiltinTypes<'env>,
    pub runtime_target: Target,
    pub link_filenames: OnceMap<String, ()>,
    pub link_frameworks: OnceMap<String, ()>,
}

impl<'env> Compiler<'env> {
    pub fn filename<'a>(&self, filename: &'a Path) -> &'a Path {
        self.project_root
            .into_iter()
            .flat_map(|root| filename.strip_prefix(root).ok())
            .next()
            .unwrap_or(filename)
    }

    pub fn target(&self, graph: ModuleGraphRef) -> Target {
        match graph {
            ModuleGraphRef::Runtime => self.runtime_target,
            ModuleGraphRef::Comptime(comptime_kind) => match comptime_kind {
                ComptimeKind::Sandbox => Target::SANDBOX,
                ComptimeKind::Target => self.runtime_target,
                ComptimeKind::Host => Target::HOST,
            },
        }
    }
}
