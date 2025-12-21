use crate::IntoLspResult;
use serde::{Serialize, de::DeserializeOwned};

pub struct NeverRespond;

impl IntoLspResult for NeverRespond {
    fn into_lsp_result(self) -> Option<impl DeserializeOwned + Serialize + Send + Sync + 'static> {
        None::<()>
    }
}
