use super::NodeRef;

#[derive(Clone, Debug)]
pub struct Cursor {
    pub position: Option<CursorPosition>,
}

#[allow(dead_code)]
impl Cursor {
    pub fn terminated() -> Self {
        Self { position: None }
    }

    pub fn value(&self) -> Option<NodeRef> {
        if let Some(position) = &self.position {
            Some(position.from)
        } else {
            None
        }
    }

    pub fn is_terminated(&self) -> bool {
        self.position.is_none()
    }

    pub fn is_live(&self) -> bool {
        self.position.is_none()
    }
}

impl From<CursorPosition> for Cursor {
    fn from(value: CursorPosition) -> Self {
        Self {
            position: Some(value),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CursorPosition {
    pub from: NodeRef,
    pub edge_index: usize,
}

impl CursorPosition {
    pub fn new(from: NodeRef, edge_index: usize) -> Self {
        Self { from, edge_index }
    }
}
