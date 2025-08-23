mod evaluate;
mod evaluate_comptime;
mod function_body;
mod function_head;
mod ty;

pub use evaluate::ResolveEvaluation;
pub use evaluate_comptime::EvaluateComptime;
pub use function_body::ResolveFunctionBody;
pub use function_head::ResolveFunctionHead;
pub use ty::ResolveType;
