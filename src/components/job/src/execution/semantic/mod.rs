mod evaluate;
mod evaluate_comptime;
mod function;
mod function_body;
mod function_head;
mod namespace;
mod namespace_items;
mod ty;
mod when;

pub use evaluate::ResolveEvaluation;
pub use evaluate_comptime::EvaluateComptime;
pub use function::ResolveFunction;
pub use function_body::ResolveFunctionBody;
pub use function_head::ResolveFunctionHead;
pub use namespace::ResolveNamespace;
pub use namespace_items::ResolveNamespaceItems;
pub use ty::ResolveType;
pub use when::ResolveWhen;
