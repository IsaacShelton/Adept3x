use crate::MaybeReady;
use derive_more::From;
use lsp_connection::LspResponse;

#[derive(Clone, Debug, From)]
pub enum LspMethodHandled {
    WontRespond,
    WillRespond(MaybeReady<LspResponse>),
}

impl LspMethodHandled {
    pub fn ready(value: LspResponse) -> Self {
        Self::WillRespond(MaybeReady::Ready(value))
    }

    pub fn pending() -> Self {
        Self::WillRespond(MaybeReady::Pending)
    }
}
