#[cfg(feature = "kernel")]
mod v1;

#[cfg(feature = "kernel")]
pub use v1::*;

#[cfg(not(feature = "kernel"))]
mod non_implementation {
    use std::sync::Arc;
    use syntax_tree::SyntaxNode;

    #[derive(Clone, Debug, Error)]
    pub enum TypingError {}
    pub type TypingResult<T> = Result<T, TypingError>;

    pub fn debug_eval(_: &Arc<SyntaxNode>) -> String {
        todo!("unpublished")
    }

    #[derive(Debug)]
    pub struct Symbol;

    #[derive(Debug)]
    pub struct Symbols {
        symbols: HashMap<String, Symbol>,
    }

    pub fn elaborate_symbols<'a>(_: impl Iterator<Item = (&'a str, Arc<SyntaxNode>)>) -> Symbols {
        todo!("unpublished")
    }
}

#[cfg(not(feature = "kernel"))]
pub use non_implementation::*;
