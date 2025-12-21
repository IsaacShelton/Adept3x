struct TextPointUtf16 {
    line: LineIndex,
    col: TextLengthUtf16,
}

impl TextPointUtf16 {
    pub fn endOf(content: &str) -> Self {
        let mut line = 0;
        let mut col = 0;

        for c in content.chars() {
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += c.len_utf16();
            }
        }

        Self {
            line: LineIndex(line),
            col: TextLengthUtf16(col),
        }
    }
}

impl Ord for TextPointUtf16 {
    fn ord(&self, other: &Self) -> Ordering {
        self.line.cmp(&other.line).then(|| self.col.cmp(&other.col))
    }
}
