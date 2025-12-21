use super::bare::BareSyntaxNode;
use std::{path::PathBuf, sync::Arc};
use text_edit::{TextEdit, TextPosition, TextRange};
use vfs::Canonical;

#[derive(Clone, Debug)]
pub struct SyntaxNode<RootData: Clone + Debug> {
    pub(crate) bare: Arc<BareSyntaxNode>,
    pub(crate) parent: Result<Arc<SyntaxNode>, RootData>,
    pub(crate) absolute_position: TextPosition,
}

impl<RootData: Clone + Debug> SyntaxNode<RootData> {
    pub fn new(
        parent: Result<Arc<SyntaxNode>, RootData>>,
        bare: Arc<BareSyntaxNode>,
        absolute_position: TextPosition,
    ) -> Arc<Self> {
        Arc::new(Self {
            bare,
            parent,
            absolute_position,
        })
    }

    pub fn parent(&self) -> Result<&Arc<Self>, &RootData> {
        self.parent.as_ref()
    }

    pub fn children(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> {
        self.bare
            .children
            .iter()
            .scan(self.absolute_position, |next_start, child| {
                let start = *next_start;
                *next_start += child.content_bytes;
                Some((start, child))
            })
            .map(|(start, child)| Self::new_arc(Ok(self.clone()), child.clone(), start))
    }

    pub fn text_range(&self) -> TextRange {
        TextRange::new(self.absolute_position, self.bare.content_bytes)
    }

    pub fn dump(self: &Arc<Self>, w: &mut impl std::io::Writer, depth: usize) {
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
