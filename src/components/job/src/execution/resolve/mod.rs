mod evaluate;
mod evaluate_comptime;
mod function_body;
mod function_head;
mod namespace;
mod structure;
mod ty;

pub use evaluate::ResolveEvaluation;
pub use evaluate_comptime::EvaluateComptime;
pub use function_body::ResolveFunctionBody;
pub use function_head::ResolveFunctionHead;
pub use namespace::ResolveNamespace;
pub use structure::ResolveStructureBody;
pub use ty::ResolveType;
