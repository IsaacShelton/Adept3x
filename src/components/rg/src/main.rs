#![allow(unused)]
use std::{
    cmp::max,
    collections::{HashMap, VecDeque},
    sync::Arc,
};

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
        let parsed = self.parse_value(content);

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

    pub fn parse_value(&mut self, mut content: &str) -> ParseResult {
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

        ParseResult {
            consumed: children.iter().map(|child| child.content_bytes).sum(),
            green: self.new_parent(GreenKind::Value, children),
        }
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

            let value = self.parse_value(content);

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

    pub fn reparse(
        &mut self,
        root: &Arc<RedNode>,
        old_content: &str,
        edit_index: usize,
        delete: usize,
        insert: &str,
    ) -> (Arc<GreenNode>, String) {
        let content = format!(
            "{}{}{}",
            &old_content[..edit_index],
            insert,
            &old_content[edit_index + delete..]
        );

        //let new_root = reparse_range(root, 0, edit_index, edit_index + delete);
        //dbg!(new_root);

        /*
        let (parent, path, start_index, end_index) =
            find_lowest_affected(&root, 0, edit_index, edit_index + delete);

        let Some(reparse) = parent.kind.can_reparse() else {
            return (self.parse_root(&content), content);
        };

        let new_node = self.reparse_as(reparse, &content[start_index..end_index]);

        dbg!(
            parent,
            path.iter().map(|x| x.0).collect::<Vec<_>>(),
            reparse,
            new_node,
        );

        let new_root = todo!();
        */

        //(new_root, content)
        todo!()
    }

    pub fn reparse_as(&mut self, reparse: Reparse, content: &str) -> ParseResult {
        match reparse {
            Reparse::Value => self.parse_value(content),
            Reparse::Array => {
                if let Some(array) = self.parse_array(content) {
                    array
                } else {
                    self.parse_value(content)
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct RedNode {
    green: Arc<GreenNode>,
    parent: Option<Arc<RedNode>>,
    absolute_position: usize,
}

impl RedNode {
    pub fn new_arc(
        parent: Option<Arc<RedNode>>,
        green: Arc<GreenNode>,
        absolute_position: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            green,
            parent,
            absolute_position,
        })
    }

    pub fn parent(&self) -> Option<&Arc<Self>> {
        self.parent.as_ref()
    }

    pub fn children(self: &Arc<Self>) -> impl Iterator<Item = Arc<RedNode>> {
        self.green
            .children
            .iter()
            .scan(self.absolute_position, |acc, child| {
                let start = *acc;
                *acc += child.content_bytes;
                Some((start, child))
            })
            .map(|(start, child)| RedNode::new_arc(Some(self.clone()), child.clone(), start))
    }
}

#[derive(Clone, Debug)]
pub struct ParseResult {
    green: Arc<GreenNode>,
    consumed: usize,
}

fn find_lowest_affected(
    node: &Arc<GreenNode>,
    node_absolute_index: usize,
    min_absolute_index: usize,
    max_absolute_index: usize,
) -> (
    Arc<GreenNode>,
    VecDeque<(usize, Arc<GreenNode>)>,
    usize,
    usize,
) {
    let mut index = node_absolute_index;

    for (i, child) in node.children.iter().enumerate() {
        let node_left = index;
        let node_right = index + child.content_bytes;

        let left_in = node_left <= min_absolute_index;
        let right_in = max_absolute_index <= node_right;
        let contained = left_in && right_in;

        if contained {
            if child.kind.can_reparse().is_none() {
                return (
                    node.clone(),
                    VecDeque::new(),
                    node_absolute_index,
                    node_absolute_index + node.content_bytes,
                );
            }

            // Fits within child
            let (parent, mut path, start_index, end_index) =
                find_lowest_affected(child, index, min_absolute_index, max_absolute_index);

            path.push_front((i, node.clone()));
            return (parent, path, start_index, end_index);
        } else if max_absolute_index < node_left {
            // Does not fit within a single child
            break;
        } else {
            index += child.content_bytes;
        }
    }

    (
        node.clone(),
        VecDeque::new(),
        node_absolute_index,
        node_absolute_index + node.content_bytes,
    )
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
    let root = RedNode::new_arc(None, green_tree.parse_root(&content), 0);
    println!("Original green tree:");
    root.green.print(0);

    let old_text = "\"parser\"";
    let edit_index = content.find(old_text).unwrap();
    let delete_len = old_text.len();
    let insert_text = "\"lexer\"";

    println!("Changing {:?} to {:?}", old_text, insert_text);

    let (new_root, new_content) =
        green_tree.reparse(&root, content, edit_index, delete_len, insert_text);

    dbg!(new_root, new_content);
}
