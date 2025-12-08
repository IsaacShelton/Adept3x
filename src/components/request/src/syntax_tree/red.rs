use super::{
    green::GreenNode,
    text::{TextEdit, TextPosition, TextRange},
};
use std::{path::PathBuf, sync::Arc};
use vfs::Canonical;

#[derive(Clone, Debug)]
pub struct RedNode {
    pub(crate) green: Arc<GreenNode>,
    pub(crate) parent: Result<Arc<RedNode>, Option<Arc<Canonical<PathBuf>>>>,
    pub(crate) absolute_position: TextPosition,
}

impl RedNode {
    pub fn new_arc(
        parent: Result<Arc<RedNode>, Option<Arc<Canonical<PathBuf>>>>,
        green: Arc<GreenNode>,
        absolute_position: TextPosition,
    ) -> Arc<Self> {
        Arc::new(Self {
            green,
            parent,
            absolute_position,
        })
    }

    pub fn parent(&self) -> Result<&Arc<Self>, Option<&Arc<Canonical<PathBuf>>>> {
        self.parent.as_ref().map_err(|err| err.as_ref())
    }

    pub fn children(self: &Arc<Self>) -> impl Iterator<Item = Arc<RedNode>> {
        self.green
            .children
            .iter()
            .scan(self.absolute_position, |next_start, child| {
                let start = *next_start;
                *next_start += child.content_bytes;
                Some((start, child))
            })
            .map(|(start, child)| RedNode::new_arc(Ok(self.clone()), child.clone(), start))
    }

    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.absolute_position, self.green.content_bytes)
    }

    pub fn print(self: &Arc<Self>, depth: usize) {
        if let Err(Some(path)) = &self.parent {
            println!("{}", path.to_string_lossy());
        }

        let padding = " ".repeat(depth * 2);
        match &self.green.text {
            Some(leaf) => {
                println!(
                    "{}@{} {:?}: {}",
                    padding,
                    self.text_range(),
                    self.green.kind,
                    leaf,
                );
            }
            None => {
                println!("{}@{} {:?}", padding, self.text_range(), self.green.kind);
                for child in self.children() {
                    child.print(depth + 1);
                }
            }
        }
    }

    #[must_use]
    pub fn reparse(
        self: &Arc<RedNode>,
        old_content: &str,
        edit: &TextEdit,
    ) -> (Arc<RedNode>, String) {
        let new_content = format!(
            "{}{}{}",
            edit.range.start().before(old_content),
            edit.replace_with,
            edit.range.end().after(old_content),
        );

        let new_green_root =
            match self.reparse_inner(&new_content, edit, TextRange::full(&new_content)) {
                ReparseInnerResult::Reparsed(green_node) => green_node,
                ReparseInnerResult::ReparseParent => GreenNode::parse_root(&new_content),
            };

        let root = RedNode::new_arc(self.parent.clone(), new_green_root, self.absolute_position);
        (root, new_content)
    }

    #[must_use]
    pub fn reparse_inner(
        self: &Arc<RedNode>,
        new_content: &str,
        edit: &TextEdit,
        new_parent_range: TextRange,
    ) -> ReparseInnerResult {
        for (i, child) in self.children().enumerate() {
            let Some(new_child_range) = child.text_range().encloses_edit(edit) else {
                continue;
            };

            match child.reparse_inner(new_content, edit, new_child_range) {
                ReparseInnerResult::Reparsed(new_child) => {
                    let new_children = Vec::from_iter(
                        self.children()
                            .map(|red| red.green.clone())
                            .take(i)
                            .chain(std::iter::once(new_child))
                            .chain(self.children().skip(i + 1).map(|red| red.green.clone())),
                    );

                    return ReparseInnerResult::Reparsed(GreenNode::new_parent(
                        self.green.kind.clone(),
                        new_children,
                    ));
                }
                ReparseInnerResult::ReparseParent => {
                    return if let Some(can_reparse) = self.green.kind.can_reparse() {
                        GreenNode::reparse_as(can_reparse, new_parent_range.of(new_content))
                    } else {
                        ReparseInnerResult::ReparseParent
                    };
                }
            }
        }

        ReparseInnerResult::ReparseParent
    }
}

#[derive(Debug)]
pub enum ReparseInnerResult {
    Reparsed(Arc<GreenNode>),
    ReparseParent,
}
