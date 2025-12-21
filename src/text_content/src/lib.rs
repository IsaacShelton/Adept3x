pub struct IncrementalTextContent {
    // We will pick our battles for now, and come back to make this a
    // rope + sum tree later on. It shouldn't matter for 99% of cases.
    // We can also have an optimization for ASCII-only files.
    content: Box<str>,
}

impl IncrementalTextContent {
    pub fn new(content: &str) -> Self {
        Self {
            content: content.into(),
        }
    }

    pub fn chars() {}

    pub fn with_edit(
        &self,
        start: LineColumnUtf16,
        end: LineColumnUtf16,
        replace_with: &str,
    ) -> Self {
        assert!(start <= end);

        let before = String::new();
        let after = String::new();

        let mut offset = LineColumnUtf16 {
            line_index: 0,
            column_index_utf16: 0,
        };

        for c in content.chars() {
            let new_offest = if c == '\n' {
                LineColumnUtf16 {
                    line_index: offset.line + 1,
                    col_utf16: 0,
                }
            } else {
                LineColumnUtf16 {
                    line: offset.line,
                    col_utf16: offset.col_utf16 + c.len_utf16(),
                }
            };

            if offset < start {
                before.push(c);
            }

            if offset >= end {
                after.push(c);
            }

            offset = new_offset;
        }

        Self {
            content: format!("{}{}{}", before, replace_with, after).into(),
        }
    }
}
