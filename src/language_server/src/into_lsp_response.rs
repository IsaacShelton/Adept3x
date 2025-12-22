use serde::{Serialize, de::DeserializeOwned};

pub trait IntoLspResult {
    fn into_lsp_result(self) -> Option<impl DeserializeOwned + Serialize + Send + Sync + 'static>;
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> IntoLspResult for T {
    fn into_lsp_result(self) -> Option<impl DeserializeOwned + Serialize + Send + Sync + 'static> {
        Some(self)
    }
}
