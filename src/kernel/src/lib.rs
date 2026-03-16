#[cfg(feature = "kernel")]
mod v1;

#[cfg(feature = "kernel")]
pub use v1::*;

#[cfg(not(feature = "kernel"))]
mod non_implementation {
    use std::sync::Arc;
    use syntax_tree::SyntaxNode;

    pub fn debug_eval(_: &Arc<SyntaxNode>) -> String {
        todo!("for testing only")
    }
}

#[cfg(not(feature = "kernel"))]
pub use non_implementation::*;
