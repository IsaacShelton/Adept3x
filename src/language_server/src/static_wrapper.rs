use crate::LspMethod;
use lsp_connection::LspConnectionState;
use std::marker::PhantomData;

pub struct Static<T>(PhantomData<T>);

impl<T: LspMethod> LspMethod for Static<T> {
    const REQUIRED_CONNECTION_STATE: LspConnectionState = T::REQUIRED_CONNECTION_STATE;
    const METHOD: &'static str = T::METHOD;
    type Params = T::Params;
    type Result = T::Result;
}

impl<T: lsp_types::request::Request> lsp_types::request::Request for Static<T> {
    type Params = T::Params;
    type Result = T::Result;
    const METHOD: &'static str = T::METHOD;
}
