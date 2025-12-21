#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRangeUtf16 {
    start: TextPointUtf16,
    end: TextPointUtf16,
}

impl TextRange {
    #[inline]
    pub fn new(start: TextPointUtf16, end: TextLengthUtf16) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn full(content: &str) -> Self {
        Self::new(TextPointUtf16(0), TextPointUtf16::endOf(content))
    }

    #[inline]
    pub fn start(&self) -> TextPointUtf16 {
        self.start
    }

    #[inline]
    pub fn end(&self) -> TextPointUtf16 {
        self.start + self.length
    }

    #[inline]
    pub fn of<'a>(&self, content: &'a str) -> &'a str {
        &content[self.start().0..self.end().0]
    }

    #[inline]
    pub fn encloses(&self, other: &Self) -> bool {
        self.start() <= other.start() && other.end() <= self.end()
    }
}

impl Display for TextRangeUtf16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start().0, self.end().0)
    }
}
