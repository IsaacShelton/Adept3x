/*
    ==================  components/job/src/continuation.rs  ===================
    List of (non-completion) continuations that tasks can perform.

    Completion continuations are handled separately by returning Ok(result),
    instead of Err(continuation).
    ---------------------------------------------------------------------------
*/

use crate::{Execution, PendingSearchVersion, io::IoRequest, module_graph::ModuleGraphRef};
use derive_more::From;
use diagnostics::ErrorDiagnostic;
use source_files::Source;

pub enum Continuation<'env> {
    // NOTE: To delay waking back up, tasks must be waited on using `ctx.suspend_on` before
    // returning. Usually this is handled indirectly via macro.
    Suspend(Execution<'env>),
    RequestIo(Execution<'env>, IoRequest),
    PendingSearch(
        Execution<'env>,
        ModuleGraphRef,
        PendingSearchVersion,
        Search<'env>,
    ),
    Error(ErrorDiagnostic),
}

impl<'env> Continuation<'env> {
    #[inline]
    pub fn suspend(execution: impl Into<Execution<'env>>) -> Self {
        Self::Suspend(execution.into())
    }
}

impl<'env> From<Result<Execution<'env>, ErrorDiagnostic>> for Continuation<'env> {
    fn from(value: Result<Execution<'env>, ErrorDiagnostic>) -> Self {
        match value {
            Ok(ok) => ok.into(),
            Err(err) => err.into(),
        }
    }
}

impl<'env> From<Execution<'env>> for Continuation<'env> {
    fn from(value: Execution<'env>) -> Self {
        Self::Suspend(value)
    }
}

impl<'env> From<ErrorDiagnostic> for Continuation<'env> {
    fn from(value: ErrorDiagnostic) -> Self {
        Self::Error(value)
    }
}

#[derive(Clone, Debug, From)]
pub enum Search<'env> {
    Func(FuncSearch<'env>),
    Namespace(NamespaceSearch<'env>),
    Type(TypeSearch<'env>),
}

impl<'env> Search<'env> {
    pub fn name(&self) -> &'env str {
        match self {
            Self::Func(func_search) => func_search.name,
            Self::Namespace(namespace_search) => namespace_search.name,
            Self::Type(type_search) => type_search.name,
        }
    }

    pub fn source(&self) -> Option<Source> {
        match self {
            Self::Func(func_search) => Some(func_search.source),
            Self::Namespace(namespace_search) => Some(namespace_search.source),
            Self::Type(type_search) => Some(type_search.source),
        }
    }

    pub fn symbol_kind_name(&self) -> Option<&'static str> {
        match self {
            Self::Func(_) => Some("function"),
            Self::Namespace(_) => Some("namespace"),
            Self::Type(_) => Some("type"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FuncSearch<'env> {
    pub name: &'env str,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct NamespaceSearch<'env> {
    pub name: &'env str,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct TypeSearch<'env> {
    pub name: &'env str,
    pub source: Source,
}
