mod green;
mod red;
mod text;

use by_address::ByAddress;
pub use green::{GreenKind, GreenNode};
pub use red::RedNode;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
pub use text::*;
use vfs::Canonical;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyntaxTree {
    root: Arc<GreenNode>,
    content: Arc<str>,
    filename: Option<Arc<Canonical<PathBuf>>>,
}

impl SyntaxTree {
    pub fn parse(content: Arc<str>, filename: Option<Arc<Canonical<PathBuf>>>) -> Self {
        Self {
            content: content.clone(),
            root: GreenNode::parse_root(&content),
            filename,
        }
    }

    pub fn print(&self) {
        self.root.print(0)
    }

    pub fn red_tree(&self) -> Arc<RedNode> {
        RedNode::new_arc(
            Err(self.filename.clone()),
            self.root.clone(),
            TextPosition(0),
        )
    }
}

impl Default for SyntaxTree {
    fn default() -> Self {
        Self {
            root: Arc::new(GreenNode {
                kind: GreenKind::Value,
                content_bytes: TextLength(0),
                children: vec![],
                text: Some("".into()),
            }),
            content: Default::default(),
            filename: None,
        }
    }
}

impl PartialEq for SyntaxTree {
    fn eq(&self, other: &Self) -> bool {
        ByAddress(self.root.as_ref()) == ByAddress(other.root.as_ref())
            && ByAddress(self.content.as_ref()) == ByAddress(other.content.as_ref())
    }
}

impl Eq for SyntaxTree {}
