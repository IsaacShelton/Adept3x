use crate::LspMethod;
use lsp_connection::LspConnectionState;
use std::marker::PhantomData;

pub struct Static<T>(PhantomData<T>);

impl<T> LspMethod for Static<T>
where
    T: LspMethod,
{
    const REQUIRED_STATE: Option<LspConnectionState> = T::REQUIRED_STATE;
    const METHOD: &'static str = T::METHOD;
    type Params = T::Params;
    type Result = T::Result;
}

impl<T> lsp_types::request::Request for Static<T>
where
    T: lsp_types::request::Request,
{
    type Params = T::Params;
    type Result = T::Result;
    const METHOD: &'static str = T::METHOD;
}
