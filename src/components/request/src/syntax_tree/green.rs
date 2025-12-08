use super::{red::ReparseInnerResult, text::TextLength};
use derive_more::IsVariant;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, IsVariant)]
pub enum GreenKind {
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

#[derive(Copy, Clone, Debug)]
pub enum Reparse {
    Value,
    Array,
}

impl GreenKind {
    pub fn can_reparse(&self) -> Option<Reparse> {
        match self {
            GreenKind::Error => None,
            GreenKind::Whitespace => None,
            GreenKind::Punct(_) => None,
            GreenKind::Null => None,
            GreenKind::True => None,
            GreenKind::False => None,
            GreenKind::Number => None,
            GreenKind::String => None,
            GreenKind::Array => Some(Reparse::Array),
            GreenKind::Value => Some(Reparse::Value),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GreenNode {
    pub(crate) kind: GreenKind,
    pub(crate) content_bytes: TextLength,
    pub(crate) children: Vec<Arc<GreenNode>>,
    pub(crate) text: Option<String>,
}

impl GreenNode {
    pub fn new_leaf(kind: GreenKind, text: String) -> Arc<GreenNode> {
        Arc::new(GreenNode {
            kind,
            content_bytes: TextLength(text.len()),
            children: vec![],
            text: Some(text),
        })
    }

    pub fn new_parent(kind: GreenKind, children: Vec<Arc<GreenNode>>) -> Arc<GreenNode> {
        Arc::new(GreenNode {
            kind,
            content_bytes: children.iter().map(|child| child.content_bytes).sum(),
            children,
            text: None,
        })
    }

    pub fn new_punct(c: char) -> Arc<GreenNode> {
        Arc::new(GreenNode {
            kind: GreenKind::Punct(c),
            content_bytes: TextLength(c.len_utf8()),
            children: vec![],
            text: Some(c.into()),
        })
    }

    pub fn new_error(rest: String) -> Arc<GreenNode> {
        Arc::new(GreenNode {
            kind: GreenKind::Error,
            content_bytes: TextLength(rest.len()),
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

    pub fn parse_root(content: &str) -> Arc<GreenNode> {
        let parsed = Self::parse_value(content);

        if parsed.consumed.0 != content.len() {
            let error = Self::new_error(content[parsed.consumed.0..].into());
            Self::new_parent(GreenKind::Value, vec![parsed.green, error])
        } else {
            parsed.green
        }
    }

    pub fn parse_whitespace(content: &str) -> Option<ParseResult> {
        let content_bytes = content
            .chars()
            .take_while(|c| c.is_ascii_whitespace())
            .map(|c| TextLength(c.len_utf8()))
            .sum::<TextLength>();

        if content_bytes.0 != 0 {
            Some(ParseResult {
                green: Self::new_leaf(GreenKind::Whitespace, content[..content_bytes.0].into()),
                consumed: content_bytes,
            })
        } else {
            None
        }
    }

    pub fn parse_keyword(content: &str, kind: GreenKind, kw: &str) -> Option<ParseResult> {
        if content.starts_with(kw) {
            Some(ParseResult {
                green: Self::new_leaf(kind, kw.into()),
                consumed: TextLength(kw.len()),
            })
        } else {
            None
        }
    }

    pub fn parse_punct(content: &str) -> Option<ParseResult> {
        for c in [',', '[', ']', '{', '}', ':'] {
            if content.starts_with(c) {
                return Some(ParseResult {
                    green: Self::new_leaf(GreenKind::Punct(c), c.into()),
                    consumed: TextLength(c.len_utf8()),
                });
            }
        }
        None
    }

    pub fn parse_number(content: &str) -> Option<ParseResult> {
        let content_bytes = content
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .map(|c| TextLength(c.len_utf8()))
            .sum::<TextLength>();

        if content_bytes.0 != 0 {
            Some(ParseResult {
                green: Self::new_leaf(GreenKind::Number, content[..content_bytes.0].into()),
                consumed: content_bytes,
            })
        } else {
            None
        }
    }

    pub fn parse_string(content: &str) -> Option<ParseResult> {
        let mut chars = content.chars();

        if chars.next() != Some('"') {
            return None;
        }

        let mut has_end = false;
        let mut content_bytes = (&mut chars)
            .take_while(|c| {
                if *c == '"' {
                    has_end = true;
                    false
                } else {
                    true
                }
            })
            .map(|c| TextLength(c.len_utf8()))
            .sum::<TextLength>();

        content_bytes += TextLength(1 + has_end as usize);

        if !has_end {
            return Some(ParseResult {
                green: Self::new_error(content[content_bytes.0..].into()),
                consumed: content_bytes,
            });
        }

        Some(ParseResult {
            green: Self::new_leaf(GreenKind::String, content[..content_bytes.0].into()),
            consumed: content_bytes,
        })
    }

    pub fn parse_value(mut content: &str) -> ParseResult {
        let mut children = Vec::new();

        if let Some(ws) = Self::parse_whitespace(&content) {
            children.push(ws.green);
            content = &content[ws.consumed.0..];
        }

        if let Some(inner) = Self::parse_array(content) {
            children.push(inner.green);
            content = &content[inner.consumed.0..];
        } else if let Some(inner) = Self::parse_string(content) {
            children.push(inner.green);
            content = &content[inner.consumed.0..];
        } else if let Some(inner) = Self::parse_number(content) {
            children.push(inner.green);
            content = &content[inner.consumed.0..];
        } else if let Some(inner) = Self::parse_keyword(content, GreenKind::True, "true") {
            children.push(inner.green);
            content = &content[inner.consumed.0..];
        } else if let Some(inner) = Self::parse_keyword(content, GreenKind::False, "false") {
            children.push(inner.green);
            content = &content[inner.consumed.0..];
        } else if let Some(inner) = Self::parse_keyword(content, GreenKind::Null, "null") {
            children.push(inner.green);
            content = &content[inner.consumed.0..];
        } else if let Some(inner) = Self::parse_punct(content) {
            children.push(inner.green);
            content = &content[inner.consumed.0..];
        } else {
            children.push(Self::new_error(content.into()));
            content = &content[content.len()..];
        }

        if let Some(ws) = Self::parse_whitespace(&content) {
            children.push(ws.green);
        }

        ParseResult {
            consumed: children.iter().map(|child| child.content_bytes).sum(),
            green: Self::new_parent(GreenKind::Value, children),
        }
    }

    pub fn parse_array(mut content: &str) -> Option<ParseResult> {
        let mut chars = content.chars();
        let mut children = Vec::new();

        if chars.next() != Some('[') {
            return None;
        }

        children.push(Self::new_punct('['));
        content = &content[1..];

        if content.starts_with(']') {
            children.push(Self::new_punct(']'));
            return Some(ParseResult {
                green: Self::new_parent(GreenKind::Array, children),
                consumed: TextLength(2),
            });
        }

        loop {
            // Missing closing bracket
            if content.is_empty() {
                children.push(Self::new_error(content.into()));

                return Some(ParseResult {
                    consumed: children.iter().map(|child| child.content_bytes).sum(),
                    green: Self::new_parent(GreenKind::Array, children),
                });
            }

            let value = Self::parse_value(content);

            children.push(value.green);
            content = &content[value.consumed.0..];

            if content.starts_with(',') {
                children.push(Self::new_punct(','));
                content = &content[1..];
                continue;
            } else if content.starts_with(']') {
                children.push(Self::new_punct(']'));
                break;
            } else {
                // Missing comma or bracket
                children.push(Self::new_error(content.into()));
                break;
            }
        }

        Some(ParseResult {
            consumed: children.iter().map(|child| child.content_bytes).sum(),
            green: Self::new_parent(GreenKind::Array, children),
        })
    }

    pub fn reparse_as(reparse: Reparse, content: &str) -> ReparseInnerResult {
        let parsed = match reparse {
            Reparse::Value => Self::parse_value(content),
            Reparse::Array => {
                if let Some(array) = Self::parse_array(content) {
                    array
                } else {
                    Self::parse_value(content)
                }
            }
        };

        if parsed.consumed.0 != content.len() {
            ReparseInnerResult::ReparseParent
        } else {
            ReparseInnerResult::Reparsed(parsed.green)
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParseResult {
    green: Arc<GreenNode>,
    consumed: TextLength,
}
