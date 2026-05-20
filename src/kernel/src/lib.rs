#[cfg(feature = "kernel")]
mod v1;

#[cfg(feature = "kernel")]
pub use v1::*;

#[cfg(not(feature = "kernel"))]
mod non_implementation {
    use std::{collections::HashMap, sync::Arc, time::Duration};
    use syntax_tree::{Binding, SyntaxNode};
    use thiserror::Error;
    use with_errors::WithErrors;

    #[derive(Clone, Debug, Error)]
    pub enum TypingError {}
    pub type TypingResult<T> = Result<T, TypingError>;

    pub fn debug_eval(_: &Arc<SyntaxNode>) -> String {
        unpublished()
    }

    #[derive(Debug)]
    pub struct Symbol;

    #[derive(Debug)]
    pub struct Symbols {
        #[allow(unused)]
        symbols: HashMap<String, Symbol>,
    }

    pub fn elaborate_symbols<'a>(_: impl Iterator<Item = Binding>) -> WithErrors<Symbols> {
        unpublished()
    }

    pub fn compile_executable(_symbols: &Symbols) -> WithErrors<()> {
        unpublished()
    }

    fn unpublished() -> ! {
        let header = "===========================================================================";
        eprintln!("{header}\nNOTE: The compiler kernel is not fully open-source yet.\n{header}\n");
        std::thread::sleep(Duration::from_secs(3));
        panic!();
    }
}

#[cfg(not(feature = "kernel"))]
pub use non_implementation::*;
