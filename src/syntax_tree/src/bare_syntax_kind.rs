#[derive(Clone, Debug, Serialize, Deserialize, IsVariant)]
pub enum BareSyntaxKind {
    Error,
    Whitespace,
    Punct(char),
    Null,
    True,
    False,
    Number,
    String,
    Array,
    Value,
}
