use crate::MaybeReady;
use derive_more::From;
use lsp_connection::LspResponse;

#[derive(Clone, Debug, From)]
pub enum Handled {
    WontRespond,
    WillRespond(MaybeReady<LspResponse>),
}

impl Handled {
    pub fn ready(value: LspResponse) -> Self {
        Self::WillRespond(MaybeReady::Ready(value))
    }

    pub fn pending() -> Self {
        Self::WillRespond(MaybeReady::Pending)
    }
}
