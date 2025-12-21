#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditUtf16<S> {
    pub range: TextRangeUtf16,
    pub replace_with: S,
}

impl TextEditUtf16<Box<str>> {
    pub fn as_ref(&self) -> TextEditUtf16<&str> {
        Self { range: self.range }
    }
}
