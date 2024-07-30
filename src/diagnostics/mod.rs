mod warning;

use crate::{show::Show, source_file_cache::SourceFileCache};
use append_only_vec::AppendOnlyVec;
use core::fmt::Debug;

pub use warning::WarningDiagnostic;

pub trait Diagnostic: Show {}

#[derive(Clone, Debug)]
pub struct DiagnosticFlags {
    pub print_without_collecting: bool,
    pub warn_padded_field: bool,
    pub warn_padded_bitfield: bool,
}

impl Default for DiagnosticFlags {
    fn default() -> Self {
        Self {
            print_without_collecting: true,
            warn_padded_field: false,
            warn_padded_bitfield: false,
        }
    }
}

pub struct Diagnostics<'a> {
    source_file_cache: &'a SourceFileCache,
    diagnostics: AppendOnlyVec<Box<dyn Diagnostic>>,
    flags: DiagnosticFlags,
}

impl<'a> Debug for Diagnostics<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Diagnostics").finish_non_exhaustive()
    }
}

impl<'a> Diagnostics<'a> {
    pub fn new(source_file_cache: &'a SourceFileCache, flags: DiagnosticFlags) -> Self {
        Self {
            source_file_cache,
            diagnostics: AppendOnlyVec::<Box<dyn Diagnostic>>::new(),
            flags,
        }
    }

    pub fn flags(&self) -> &DiagnosticFlags {
        &self.flags
    }

    pub fn push(&self, diagnostic: impl Diagnostic + 'static) {
        if self.flags.print_without_collecting {
            self.print(&diagnostic);
        } else {
            self.diagnostics.push(Box::new(diagnostic));
        }
    }

    pub fn print_all(&self) {
        for diagnostic in self.diagnostics.iter() {
            self.print(&**diagnostic);
        }
    }

    pub fn print(&self, diagnostic: &dyn Diagnostic) {
        let mut message = String::new();

        diagnostic
            .show(&mut message, self.source_file_cache)
            .expect("show error message");

        eprintln!("{message}");
    }
}
