use text_edit::TextLengthUtf16;
use util_data_unit::ByteUnits;

#[derive(Clone, Debug)]
pub struct DocumentLine {
    pub(crate) content: String,
}

impl DocumentLine {
    pub fn new(content: String) -> Self {
        Self { content: content }
    }

    pub fn translate_utf16_index(&self, index: TextLengthUtf16) -> ByteUnits {
        self.content
            .chars()
            .scan((0, 0), |st, c| {
                let start = *st;
                st.0 += c.len_utf8();
                st.1 += c.len_utf16();
                Some(start)
            })
            .find(|(_utf8, utf16)| *utf16 >= index.0)
            .map(|(utf8, _)| utf8)
            .unwrap_or(self.content.len())
            .try_into()
            .unwrap()
    }

    pub fn append(&mut self, text: &str) {
        self.content.push_str(text)
    }

    pub fn insert(&mut self, index: usize, text: &str) {
        self.content.insert_str(index, text);
    }

    pub fn delete_after(&mut self, index: usize) {
        self.content.truncate(index);
    }
}
