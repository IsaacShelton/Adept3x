use serde::{Serialize, de::DeserializeOwned};

pub trait IntoLspResult {
    fn into_lsp_result(self) -> Option<impl DeserializeOwned + Serialize + Send + Sync + 'static>;
}

impl<T> IntoLspResult for T
where
    T: DeserializeOwned + Serialize + Send + Sync + 'static,
{
    fn into_lsp_result(self) -> Option<impl DeserializeOwned + Serialize + Send + Sync + 'static> {
        Some(self)
    }
}
