use crate::{IntoLspResult, NeverRespond};
use lsp_connection::LspConnectionState;
use serde::{Serialize, de::DeserializeOwned};

pub trait LspMethod {
    const REQUIRED_STATE: Option<LspConnectionState>;
    const METHOD: &'static str;
    type Params: DeserializeOwned + Serialize + Send + Sync + 'static;
    type Result: IntoLspResult;
}

impl LspMethod for lsp_types::request::Initialize {
    const REQUIRED_STATE: Option<LspConnectionState> = Some(LspConnectionState::Started);
    const METHOD: &'static str = <Self as lsp_types::request::Request>::METHOD;
    type Params = <Self as lsp_types::request::Request>::Params;
    type Result = <Self as lsp_types::request::Request>::Result;
}

impl LspMethod for lsp_types::request::Shutdown {
    const REQUIRED_STATE: Option<LspConnectionState> = Some(LspConnectionState::Initialized);
    const METHOD: &'static str = <Self as lsp_types::request::Request>::METHOD;
    type Params = <Self as lsp_types::request::Request>::Params;
    type Result = <Self as lsp_types::request::Request>::Result;
}

impl LspMethod for lsp_types::request::Completion {
    const REQUIRED_STATE: Option<LspConnectionState> = Some(LspConnectionState::Initialized);
    const METHOD: &'static str = <Self as lsp_types::request::Request>::METHOD;
    type Params = <Self as lsp_types::request::Request>::Params;
    type Result = <Self as lsp_types::request::Request>::Result;
}

impl LspMethod for lsp_types::notification::Initialized {
    const REQUIRED_STATE: Option<LspConnectionState> = Some(LspConnectionState::Initialized);
    const METHOD: &'static str = <Self as lsp_types::notification::Notification>::METHOD;
    type Params = <Self as lsp_types::notification::Notification>::Params;
    type Result = NeverRespond;
}

impl LspMethod for lsp_types::notification::DidOpenTextDocument {
    const REQUIRED_STATE: Option<LspConnectionState> = Some(LspConnectionState::Initialized);
    const METHOD: &'static str = <Self as lsp_types::notification::Notification>::METHOD;
    type Params = <Self as lsp_types::notification::Notification>::Params;
    type Result = NeverRespond;
}
