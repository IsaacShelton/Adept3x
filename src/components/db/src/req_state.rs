use crate::Artifact;

pub enum ReqState {
    Initial,
    Complete(Artifact),
}

impl ReqState {
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete(..))
    }

    pub fn as_complete(&self) -> Option<&Artifact> {
        match self {
            ReqState::Complete(artifact) => Some(artifact),
            _ => None,
        }
    }
}
