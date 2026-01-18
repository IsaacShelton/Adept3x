use crate::BareSyntaxNode;
use std::{fmt::Debug, sync::Arc};
use text_edit::{TextPointRangeUtf16, TextPointUtf16};

pub struct SyntaxNode<Root: Clone + Debug> {
    pub(crate) bare: Arc<BareSyntaxNode>,
    pub(crate) parent: Result<Arc<SyntaxNode<Root>>, Root>,
    pub(crate) start: TextPointUtf16,
}

impl<Root: Clone + Debug> SyntaxNode<Root> {
    pub fn new(
        parent: Result<Arc<Self>, Root>,
        bare: Arc<BareSyntaxNode>,
        start: TextPointUtf16,
    ) -> Arc<Self> {
        Arc::new(Self {
            parent,
            bare,
            start,
        })
    }

    pub fn parent(&self) -> Result<&Arc<Self>, &Root> {
        self.parent.as_ref()
    }

    pub fn text_range(&self) -> TextPointRangeUtf16 {
        TextPointRangeUtf16::new(self.start, self.start + self.bare.text_point_diff_utf16)
    }

    pub fn children(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> {
        self.bare
            .children
            .iter()
            .scan(self.start, |next_start, child| {
                let start = *next_start;
                *next_start += child.text_point_diff_utf16;
                Some((start, child))
            })
            .map(|(start, child)| Self::new(Ok(self.clone()), child.clone(), start))
    }

    pub fn dump(
        self: &Arc<Self>,
        w: &mut impl std::io::Write,
        depth: usize,
    ) -> std::io::Result<()> {
        if let Err(_) = &self.parent {
            writeln!(w, "<root>")?;
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
                )?;
            }
            None => {
                writeln!(w, "{}@{} {:?}", padding, self.text_range(), self.bare.kind)?;
                for child in self.children() {
                    child.dump(w, depth + 1)?;
                }
            }
        }

        Ok(())
    }
}
