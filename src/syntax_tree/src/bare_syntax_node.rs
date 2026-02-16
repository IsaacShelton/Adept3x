use crate::BareSyntaxKind;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use text_edit::TextPointDiffUtf16;
use token::Punct;
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

    pub fn new_punct(punct: Punct) -> Arc<BareSyntaxNode> {
        Arc::new(BareSyntaxNode {
            kind: BareSyntaxKind::Punct(punct),
            content_bytes: ByteUnits::of(punct.len() as u64),
            text_point_diff_utf16: TextPointDiffUtf16::of_str(punct.as_str()),
            children: vec![],
            text: Some(punct.to_string()),
        })
    }

    pub fn new_error(rest: String, description: impl Into<String>) -> Arc<BareSyntaxNode> {
        Arc::new(BareSyntaxNode {
            kind: BareSyntaxKind::Error {
                description: description.into(),
            },
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

    pub fn kind(&self) -> &BareSyntaxKind {
        &self.kind
    }

    pub fn children(&self) -> impl Iterator<Item = &Arc<BareSyntaxNode>> {
        self.children.iter()
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
