use crate::BareSyntaxKind;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use text_edit::TextPointDiffUtf16;
use util_data_unit::ByteUnits;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BareSyntaxNode {
    pub(crate) kind: BareSyntaxKind,
    pub(crate) content_bytes: ByteUnits,
    pub(crate) text_point_diff_utf16: TextPointDiffUtf16,
    pub(crate) children: Vec<Arc<BareSyntaxNode>>,
    pub(crate) text: Option<String>,
}

impl BareSyntaxNode {
    pub fn new_leaf(kind: BareSyntaxKind, text: String) -> Arc<BareSyntaxNode> {
        Arc::new(BareSyntaxNode {
            kind,
            content_bytes: ByteUnits::of(text.len().try_into().unwrap()),
            text_point_diff_utf16: TextPointDiffUtf16::of_str(&text),
            children: vec![],
            text: Some(text),
        })
    }

    pub fn new_parent(
        kind: BareSyntaxKind,
        children: Vec<Arc<BareSyntaxNode>>,
    ) -> Arc<BareSyntaxNode> {
        Arc::new(BareSyntaxNode {
            kind,
            content_bytes: children.iter().map(|child| child.content_bytes).sum(),
            text_point_diff_utf16: children
                .iter()
                .map(|child| child.text_point_diff_utf16)
                .sum(),
            children,
            text: None,
        })
    }

    pub fn new_punct(c: char) -> Arc<BareSyntaxNode> {
        Arc::new(BareSyntaxNode {
            kind: BareSyntaxKind::Punct(c),
            content_bytes: ByteUnits::of(1),
            text_point_diff_utf16: TextPointDiffUtf16::of_char(c),
            children: vec![],
            text: Some(c.into()),
        })
    }

    pub fn new_error(rest: String) -> Arc<BareSyntaxNode> {
        Arc::new(BareSyntaxNode {
            kind: BareSyntaxKind::Error,
            content_bytes: ByteUnits::of(rest.len().try_into().unwrap()),
            text_point_diff_utf16: TextPointDiffUtf16::of_str(&rest),
            children: vec![],
            text: Some(rest),
        })
    }

    pub fn print(&self, depth: usize) {
        let padding = " ".repeat(depth * 2);
        match &self.text {
            Some(leaf) => {
                println!("{}{:?}: {:?}", padding, self.kind, leaf);
            }
            None => {
                println!("{}{:?}", padding, self.kind);
                for child in self.children.iter() {
                    child.print(depth + 1);
                }
            }
        }
    }

    pub fn flatten(&self) -> String {
        let mut builder = String::with_capacity(128);
        self.flatten_into(&mut builder);
        builder
    }

    pub fn flatten_into(&self, s: &mut String) {
        self.text.as_ref().map(|text| s.push_str(text));

        for child in self.children.iter() {
            child.flatten_into(s);
        }
    }
}
