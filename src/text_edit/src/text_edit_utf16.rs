use crate::TextPointRangeUtf16;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditUtf16 {
    pub range: TextPointRangeUtf16,
    pub replace_with: Box<str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditOrFullUtf16 {
    pub range: Option<TextPointRangeUtf16>,
    pub replace_with: Box<str>,
}

impl TextEditOrFullUtf16 {
    pub fn as_text_edit(self) -> Result<TextEditUtf16, Box<str>> {
        match self.range {
            Some(range) => Ok(TextEditUtf16 {
                range,
                replace_with: self.replace_with,
            }),
            None => Err(self.replace_with),
        }
    }
}

impl From<lsp_types::TextDocumentContentChangeEvent> for TextEditOrFullUtf16 {
    fn from(change: lsp_types::TextDocumentContentChangeEvent) -> Self {
        Self {
            range: change.range.map(|range| TextPointRangeUtf16::from(range)),
            replace_with: change.text.into(),
        }
    }
}
