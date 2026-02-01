use crate::BareSyntaxNode;
use std::{fmt::Debug, sync::Arc};
use text_edit::{TextPointRangeUtf16, TextPointUtf16};

#[derive(Debug)]
pub struct SyntaxNode {
    pub(crate) bare: Arc<BareSyntaxNode>,
    pub(crate) parent: Option<Arc<SyntaxNode>>,
    pub(crate) start: TextPointUtf16,
}

impl SyntaxNode {
    pub fn new(
        parent: Option<Arc<Self>>,
        bare: Arc<BareSyntaxNode>,
        start: TextPointUtf16,
    ) -> Arc<Self> {
        Arc::new(Self {
            parent,
            bare,
            start,
        })
    }

    pub fn parent(&self) -> Option<&Arc<Self>> {
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
            .map(|(start, child)| Self::new(Some(self.clone()), child.clone(), start))
    }

    pub fn dump(
        self: &Arc<Self>,
        w: &mut impl std::io::Write,
        depth: usize,
    ) -> std::io::Result<()> {
        if self.parent.is_none() {
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
