mod line;
mod position;
mod range;
mod unit_tests;

use crate::line::DocumentLine;
use itertools::Itertools;
pub use position::DocumentPosition;
pub use range::DocumentRange;
use text_edit::{
    LineIndex, TextEditOrFullUtf16, TextEditUtf16, TextPointRangeUtf16, TextPointUtf16,
};
use util_data_unit::ByteUnits;

#[derive(Clone, Debug)]
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

    pub fn apply_utf16_text_edit_or_full(&mut self, text_edit_or_full: TextEditOrFullUtf16) {
        match text_edit_or_full.as_text_edit() {
            Ok(text_edit) => self.apply_utf16_text_edit(text_edit),
            Err(full_content) => *self = Self::new(full_content),
        }
    }

    pub fn full_range(&self) -> DocumentRange {
        let last_line = self.lines.last().unwrap();
        DocumentRange {
            start: DocumentPosition {
                line: LineIndex(0),
                index: ByteUnits::of(0),
            },
            end: DocumentPosition {
                line: LineIndex(self.lines.len().checked_sub(1).unwrap()),
                index: ByteUnits::of(last_line.content.len().try_into().unwrap()),
            },
        }
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
        // Get the portions of edited line before & after the edit position
        let start_line = self.lines.get_mut(position.line.0).unwrap();
        let prefix = &start_line.content[..position.index.bytes() as usize];
        let suffix = &start_line.content[position.index.bytes() as usize..];

        // Split replacement text into lines, and add the prefix and suffix back in
        let mut new_lines = text.split('\n').map(String::from).collect_vec();
        new_lines.first_mut().unwrap().insert_str(0, prefix);
        new_lines.last_mut().unwrap().push_str(suffix);

        // Replace the affected line with the new lines
        self.lines.splice(
            position.line.0..(position.line.0 + 1),
            new_lines
                .into_iter()
                .map(|text| DocumentLine::new(text.into())),
        );
    }

    pub fn chars(&self) -> impl Iterator<Item = char> {
        Itertools::intersperse(self.lines.iter().map(|line| line.content.as_str()), "\n")
            .flat_map(|x| x.chars())
    }
}
