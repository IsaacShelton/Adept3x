use crate::{IntoLspResult, NeverRespond};
use lsp_connection::LspConnectionState;
use serde::{Serialize, de::DeserializeOwned};

pub trait LspMethod {
    const REQUIRED_STATE: Option<LspConnectionState>;
    const METHOD: &'static str;
    type Params: DeserializeOwned + Serialize + Send + Sync + 'static;
    type Result: IntoLspResult;
}

macro_rules! request {
    ($request_ty: ident) => {
        request!($request_ty, LspConnectionState::Initialized);
    };
    ($notif_ty: ident, $state:expr) => {
        impl LspMethod for lsp_types::request::$notif_ty {
            const REQUIRED_STATE: Option<LspConnectionState> = Some($state);
            const METHOD: &'static str = <Self as lsp_types::request::Request>::METHOD;
            type Params = <Self as lsp_types::request::Request>::Params;
            type Result = <Self as lsp_types::request::Request>::Result;
        }
    };
}

macro_rules! notification {
    ($notif_ty: ident) => {
        notification!($notif_ty, LspConnectionState::Initialized);
    };
    ($notif_ty: ident, $state:expr) => {
        impl LspMethod for lsp_types::notification::$notif_ty {
            const REQUIRED_STATE: Option<LspConnectionState> = Some($state);
            const METHOD: &'static str = <Self as lsp_types::notification::Notification>::METHOD;
            type Params = <Self as lsp_types::notification::Notification>::Params;
            type Result = NeverRespond;
        }
    };
}

request!(Initialize, LspConnectionState::Started);
request!(Shutdown);
request!(Completion);

notification!(Initialized, LspConnectionState::Started);
notification!(DidOpenTextDocument);
notification!(SetTrace);
