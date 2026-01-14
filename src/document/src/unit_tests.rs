#![allow(unused_imports)]

use crate::{Document, DocumentPosition, DocumentRange};
use text_edit::{LineIndex, TextLengthUtf16, TextPointUtf16};
use util_data_unit::ByteUnits;

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
