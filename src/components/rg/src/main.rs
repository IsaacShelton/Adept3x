#![allow(unused)]
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct GreenNodeId(usize);

#[derive(Clone, Debug)]
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
    KeyValue,
    Object,
    Value,
}

#[derive(Clone, Debug)]
pub struct GreenNode {
    id: GreenNodeId,
    kind: GreenKind,
    content_bytes: usize,
    children: Vec<Arc<GreenNode>>,
    text: Option<String>,
}

impl GreenNode {
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
}

#[derive(Clone, Debug, Default)]
pub struct GreenTree {
    nodes: HashMap<GreenNodeId, Arc<GreenNode>>,
    next_green_node_id: usize,
}

impl GreenTree {
    fn new_id(&mut self) -> GreenNodeId {
        let id = GreenNodeId(self.next_green_node_id);
        self.next_green_node_id += 1;
        id
    }

    pub fn new_leaf(&mut self, kind: GreenKind, text: String) -> Arc<GreenNode> {
        Arc::new(GreenNode {
            id: self.new_id(),
            kind,
            content_bytes: text.len(),
            children: vec![],
            text: Some(text),
        })
    }

    pub fn new_parent(&mut self, kind: GreenKind, children: Vec<Arc<GreenNode>>) -> Arc<GreenNode> {
        Arc::new(GreenNode {
            id: self.new_id(),
            kind,
            content_bytes: children.iter().map(|child| child.content_bytes).sum(),
            children,
            text: None,
        })
    }

    pub fn new_punct(&mut self, c: char) -> Arc<GreenNode> {
        Arc::new(GreenNode {
            id: self.new_id(),
            kind: GreenKind::Punct(c),
            content_bytes: c.len_utf8(),
            children: vec![],
            text: Some(c.into()),
        })
    }

    pub fn new_error(&mut self, rest: String) -> Arc<GreenNode> {
        Arc::new(GreenNode {
            id: self.new_id(),
            kind: GreenKind::Error,
            content_bytes: rest.len(),
            children: vec![],
            text: Some(rest),
        })
    }

    pub fn parse_root(&mut self, content: &str) -> Arc<GreenNode> {
        let Some(parsed) = self.parse_value(content) else {
            return self.new_error(content.into());
        };

        if parsed.consumed != content.len() {
            let error = self.new_error(content[parsed.consumed..].into());
            self.new_parent(GreenKind::Value, vec![parsed.green, error])
        } else {
            parsed.green
        }
    }

    pub fn parse_whitespace(&mut self, content: &str) -> Option<ParseResult> {
        let content_bytes = content
            .chars()
            .take_while(|c| c.is_ascii_whitespace())
            .map(|c| c.len_utf8())
            .sum();

        if content_bytes != 0 {
            Some(ParseResult {
                green: self.new_leaf(GreenKind::Whitespace, content[..content_bytes].into()),
                consumed: content_bytes,
            })
        } else {
            None
        }
    }

    pub fn parse_keyword(
        &mut self,
        content: &str,
        kind: GreenKind,
        kw: &str,
    ) -> Option<ParseResult> {
        if content.starts_with(kw) {
            Some(ParseResult {
                green: self.new_leaf(kind, kw.into()),
                consumed: kw.len(),
            })
        } else {
            None
        }
    }

    pub fn parse_punct(&mut self, content: &str) -> Option<ParseResult> {
        for c in [',', '[', ']', '{', '}', ':'] {
            if content.starts_with(c) {
                return Some(ParseResult {
                    green: self.new_leaf(GreenKind::Punct(c), c.into()),
                    consumed: c.len_utf8(),
                });
            }
        }
        None
    }

    pub fn parse_number(&mut self, content: &str) -> Option<ParseResult> {
        let content_bytes = content
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .map(|c| c.len_utf8())
            .sum();

        if content_bytes != 0 {
            Some(ParseResult {
                green: self.new_leaf(GreenKind::Number, content[..content_bytes].into()),
                consumed: content_bytes,
            })
        } else {
            None
        }
    }

    pub fn parse_string(&mut self, content: &str) -> Option<ParseResult> {
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
            .map(|c| c.len_utf8())
            .sum();
        content_bytes += 1 + has_end as usize;

        if !has_end {
            return Some(ParseResult {
                green: self.new_error(content[content_bytes..].into()),
                consumed: content_bytes,
            });
        }

        Some(ParseResult {
            green: self.new_leaf(GreenKind::String, content[..content_bytes].into()),
            consumed: content_bytes,
        })
    }

    pub fn parse_value(&mut self, mut content: &str) -> Option<ParseResult> {
        let mut children = Vec::new();

        if let Some(ws) = self.parse_whitespace(&content) {
            children.push(ws.green);
            content = &content[ws.consumed..];
        }

        if let Some(inner) = self.parse_array(content) {
            children.push(inner.green);
            content = &content[inner.consumed..];
        } else if let Some(inner) = self.parse_string(content) {
            children.push(inner.green);
            content = &content[inner.consumed..];
        } else if let Some(inner) = self.parse_number(content) {
            children.push(inner.green);
            content = &content[inner.consumed..];
        } else if let Some(inner) = self.parse_keyword(content, GreenKind::True, "true") {
            children.push(inner.green);
            content = &content[inner.consumed..];
        } else if let Some(inner) = self.parse_keyword(content, GreenKind::False, "false") {
            children.push(inner.green);
            content = &content[inner.consumed..];
        } else if let Some(inner) = self.parse_keyword(content, GreenKind::Null, "null") {
            children.push(inner.green);
            content = &content[inner.consumed..];
        } else if let Some(inner) = self.parse_punct(content) {
            children.push(inner.green);
            content = &content[inner.consumed..];
        } else {
            children.push(self.new_error(content.into()));
            content = &content[content.len()..];
        }

        if let Some(ws) = self.parse_whitespace(&content) {
            children.push(ws.green);
            content = &content[ws.consumed..];
        }

        Some(ParseResult {
            consumed: children.iter().map(|child| child.content_bytes).sum(),
            green: self.new_parent(GreenKind::Value, children),
        })
    }

    pub fn parse_array(&mut self, mut content: &str) -> Option<ParseResult> {
        let mut chars = content.chars();
        let mut children = Vec::new();

        if chars.next() != Some('[') {
            return None;
        }

        children.push(self.new_punct('['));
        content = &content[1..];

        if content.starts_with(']') {
            children.push(self.new_punct(']'));
            return Some(ParseResult {
                green: self.new_parent(GreenKind::Array, children),
                consumed: 2,
            });
        }

        loop {
            // Missing closing bracket
            if content.is_empty() {
                children.push(self.new_error(content.into()));

                return Some(ParseResult {
                    consumed: children.iter().map(|child| child.content_bytes).sum(),
                    green: self.new_parent(GreenKind::Array, children),
                });
            }

            let Some(value) = self.parse_value(content) else {
                break;
            };

            children.push(value.green);
            content = &content[value.consumed..];

            if content.starts_with(',') {
                children.push(self.new_punct(','));
                content = &content[1..];
                continue;
            } else if content.starts_with(']') {
                children.push(self.new_punct(']'));
                content = &content[1..];
                break;
            } else {
                // Missing comma or bracket
                children.push(self.new_error(content.into()));
                content = &content[content.len()..];
                break;
            }
        }

        Some(ParseResult {
            consumed: children.iter().map(|child| child.content_bytes).sum(),
            green: self.new_parent(GreenKind::Array, children),
        })
    }
}

#[derive(Clone, Debug)]
pub struct RedNode {
    green: Arc<GreenNode>,
    parent: Option<Arc<RedNode>>,
    absolute_position: usize,
}

#[derive(Clone, Debug)]
pub struct ParseResult {
    green: Arc<GreenNode>,
    consumed: usize,
}

fn main() {
    let content = r#"
      ["rust", "parser",

      1034,
      "ending"

      ]
    "#;

    let mut green_tree = GreenTree::default();
    println!("Original source: {}", content);
    let root = green_tree.parse_root(&content);
    println!("Original green tree:");
    root.print(0);
}
