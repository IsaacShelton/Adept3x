use itertools::Itertools;
use text_edit::{LineIndex, TextEditUtf16, TextLengthUtf16, TextPointRangeUtf16, TextPointUtf16};
use util_data_unit::ByteUnits;

pub struct Document {
    lines: Vec<DocumentLine>,
}

impl Document {
    pub fn new(content: Box<str>) -> Self {
        let lines = content
            .split('\n')
            .map(|s| DocumentLine::new(s.into()))
            .collect();
        Self { lines }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn translate_utf16_point(&self, point: TextPointUtf16) -> DocumentPosition {
        let line = self.lines.get(point.line.0).unwrap();

        DocumentPosition {
            line: point.line,
            index: line.translate_utf16_index(point.col),
        }
    }

    pub fn translate_utf16_point_range(&self, range: TextPointRangeUtf16) -> DocumentRange {
        let start = self.translate_utf16_point(range.start());
        let end = self.translate_utf16_point(range.end());
        DocumentRange { start, end }
    }

    pub fn apply_utf16_text_edit(&mut self, text_edit: TextEditUtf16) {
        let range = self.translate_utf16_point_range(text_edit.range);
        self.delete(range);
        self.insert(range.start, &text_edit.replace_with);
    }

    pub fn delete(&mut self, range: DocumentRange) {
        // Handle single line edit as special case
        if range.start.line == range.end.line {
            let line = self.lines.get_mut(range.start.line.0).unwrap();
            line.content.replace_range(
                range.start.index.bytes() as usize..range.end.index.bytes() as usize,
                "",
            );
            return;
        }

        // Get start and end lines separately
        let [start_line, end_line] = self
            .lines
            .get_disjoint_mut([range.start.line.0, range.end.line.0])
            .unwrap();

        // Remove deleted characters on first line
        start_line.delete_after(range.start.index.bytes() as usize);

        // Append characters acquired from last line onto end of first line
        start_line.append(&end_line.content[range.end.index.bytes() as usize..]);

        // Delete (start_line+1..end_line) lines
        let next_line = range.start.line.0 + 1;
        let lines_to_delete = range.end.line.0 - range.start.line.0;
        self.lines
            .splice(next_line..next_line + lines_to_delete, []);
    }

    pub fn insert(&mut self, position: DocumentPosition, text: &str) {
        let mut added_lines = text.split('\n');
        let start_line = self.lines.get_mut(position.line.0).unwrap();

        if let Some(text) = added_lines.next() {
            start_line.insert(position.index.bytes() as usize, text);
        }

        let next_line = position.line.0 + 1;
        self.lines.splice(
            next_line..next_line,
            added_lines.map(|text| DocumentLine::new(text.into())),
        );
    }

    pub fn chars(&self) -> impl Iterator<Item = char> {
        Itertools::intersperse(self.lines.iter().map(|line| line.content.as_str()), "\n")
            .flat_map(|x| x.chars())
    }
}

pub struct DocumentLine {
    content: String,
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

    pub fn prepend(&mut self, text: &str) {
        self.content.insert_str(0, text);
    }

    pub fn insert(&mut self, index: usize, text: &str) {
        self.content.insert_str(index, text);
    }

    pub fn delete_after(&mut self, index: usize) {
        self.content.truncate(index);
    }

    pub fn delete_before(&mut self, index: usize) {
        self.content.replace_range(0..index, "");
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentPosition {
    line: LineIndex,
    index: ByteUnits,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DocumentRange {
    start: DocumentPosition,
    end: DocumentPosition,
}

#[test]
fn test_point_translate_utf16() {
    let message: &str = "Hello world! 你好世界!丽 🎄︎聖誕快樂! 😀!!!";
    let document = Document::new(message.into());

    for (utf8_byte_index, utf16_byte_index) in message.chars().scan((0, 0), |st, c| {
        let start = *st;
        st.0 += c.len_utf8();
        st.1 += c.len_utf16();
        Some(start)
    }) {
        assert_eq!(
            document.translate_utf16_point(TextPointUtf16 {
                line: LineIndex(0),
                col: TextLengthUtf16(utf16_byte_index),
            }),
            DocumentPosition {
                line: LineIndex(0),
                index: ByteUnits::of(utf8_byte_index.try_into().unwrap()),
            }
        );
    }
}

#[test]
fn test_single_line_delete() {
    let message_before = "Hello world! 你好世界!丽 🎄︎聖誕快樂! 😀!!!";
    let message_after = "Hello world! 世界!丽 🎄︎聖誕快樂! 😀!!!";
    let mut document = Document::new(message_before.into());

    let line = LineIndex(0);
    let range = DocumentRange {
        start: document.translate_utf16_point(TextPointUtf16 {
            line,
            col: TextLengthUtf16(13),
        }),
        end: document.translate_utf16_point(TextPointUtf16 {
            line,
            col: TextLengthUtf16(15),
        }),
    };
    document.delete(range);

    assert_eq!(String::from_iter(document.chars()), message_after);
}

#[test]
fn test_single_line_insert() {
    let insert = "你好";
    let message_before = "Hello world! 世界!丽 🎄︎聖誕快樂! 😀!!!";
    let message_after = "Hello world! 你好世界!丽 🎄︎聖誕快樂! 😀!!!";

    let mut document = Document::new(message_before.into());
    document.insert(
        document.translate_utf16_point(TextPointUtf16 {
            line: LineIndex(0),
            col: TextLengthUtf16(13),
        }),
        insert,
    );

    assert_eq!(String::from_iter(document.chars()), message_after);
}

#[test]
fn test_add_line() {
    let insert = "\n你好";
    let message_before = "Hello world!";
    let message_after = "Hello world!\n你好";

    let mut document = Document::new(message_before.into());
    document.insert(
        document.translate_utf16_point(TextPointUtf16 {
            line: LineIndex(0),
            col: TextLengthUtf16(13),
        }),
        insert,
    );

    assert_eq!(String::from_iter(document.chars()), message_after);
}

#[test]
fn test_delete_line() {
    let message_before = "Hello world!\n你好";
    let message_after = "Hello world!";

    let mut document = Document::new(message_before.into());
    document.delete(DocumentRange {
        start: document.translate_utf16_point(TextPointUtf16 {
            line: LineIndex(0),
            col: TextLengthUtf16(13),
        }),
        end: document.translate_utf16_point(TextPointUtf16 {
            line: LineIndex(1),
            col: TextLengthUtf16(2),
        }),
    });

    assert_eq!(String::from_iter(document.chars()), message_after);
}

#[test]
fn test_delete_part_across_lines() {
    let message_before = "Hello world!\n你好";
    let message_after = "Hello world!好";

    let mut document = Document::new(message_before.into());
    document.delete(DocumentRange {
        start: document.translate_utf16_point(TextPointUtf16 {
            line: LineIndex(0),
            col: TextLengthUtf16(13),
        }),
        end: document.translate_utf16_point(TextPointUtf16 {
            line: LineIndex(1),
            col: TextLengthUtf16(1),
        }),
    });

    assert_eq!(String::from_iter(document.chars()), message_after);
}

#[test]
fn test_delete_part_across_lines_2() {
    let message_before = "Hello world!\n你好\n世界!";
    let message_after = "Hello world!界!";

    let mut document = Document::new(message_before.into());
    document.delete(DocumentRange {
        start: document.translate_utf16_point(TextPointUtf16 {
            line: LineIndex(0),
            col: TextLengthUtf16(13),
        }),
        end: document.translate_utf16_point(TextPointUtf16 {
            line: LineIndex(2),
            col: TextLengthUtf16(1),
        }),
    });

    assert_eq!(String::from_iter(document.chars()), message_after);
}
