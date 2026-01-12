/*
use super::BareSyntaxNode;
use std::{fmt::Debug, sync::Arc};
use util_data_unit::ByteUnits;

use vfs::Canonical;

#[derive(Clone, Debug)]
pub struct SyntaxNode<Root: Clone + Debug> {
    pub(crate) bare: Arc<BareSyntaxNode>,
    pub(crate) parent: Result<Arc<SyntaxNode<Root>>, Root>,
    pub(crate) start_in_utf16: TextPointUtf16,
}

impl<Root: Clone + Debug> SyntaxNode<Root> {
    pub fn new(
        parent: Result<Arc<Self>, Root>,
        bare: Arc<BareSyntaxNode>,
        start_in_byptes: ByteUnits,
    ) -> Arc<Self> {
        Arc::new(Self {
            bare,
            parent,
            start_in_bytes: absolute_position,
        })
    }

    pub fn parent(&self) -> Result<&Arc<Self>, &Root> {
        self.parent.as_ref()
    }

    pub fn children(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> {
        self.bare
            .children
            .iter()
            .scan(self.start_in_bytes, |next_start, child| {
                let start = *next_start;
                *next_start += child.content_bytes;
                Some((start, child))
            })
            .map(|(start, child)| Self::new(Ok(self.clone()), child.clone(), start))
    }

    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.start_in_bytes, self.bare.content_len_utf16)
    }

    pub fn dump(self: &Arc<Self>, w: &mut impl std::io::Write, depth: usize) {
        if let Err(Some(path)) = &self.parent {
            writeln!(w, "{}", path.to_string_lossy());
        }

        let padding = " ".repeat(depth * 2);
        match &self.bare.text {
            Some(leaf) => {
                writeln!(
                    w,
                    "{}@{} {:?}: {}",
                    padding,
                    self.text_range(),
                    self.bare.kind,
                    leaf,
                );
            }
            None => {
                writeln!(w, "{}@{} {:?}", padding, self.text_range(), self.bare.kind);
                for child in self.children() {
                    child.print(depth + 1);
                }
            }
        }
    }
}
*/
