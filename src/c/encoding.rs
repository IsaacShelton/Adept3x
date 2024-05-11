
#[derive(Clone, Debug, Hash, PartialEq)]
pub enum Encoding {
    Default,
    Utf8,  // 'u8'
    Utf16, // 'u'
    Utf32, // 'U'
    Wide,  // 'L'
}
