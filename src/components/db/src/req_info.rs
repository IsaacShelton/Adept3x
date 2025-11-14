use crate::{Artifact, NumberedRevision, Req, ReqState, artifact};
use std_ext::SmallVec4;

pub struct ReqInfo<REV> {
    pub last_used: REV,
    pub last_computed: REV,
    pub state: Option<ReqState>,
    pub prev_artifact: Option<Artifact>,
    pub dependencies: SmallVec4<Req>,
}

impl<REV: Clone> ReqInfo<REV> {
    pub fn initial(rev: REV) -> Self {
        Self {
            last_used: rev.clone(),
            last_computed: rev,
            state: Some(ReqState::Initial),
            dependencies: Default::default(),
            prev_artifact: None,
        }
    }

    pub fn complete(rev: REV, artifact: Artifact) -> Self {
        Self {
            last_used: rev.clone(),
            last_computed: rev,
            state: Some(ReqState::Complete(artifact)),
            dependencies: Default::default(),
            prev_artifact: None,
        }
    }
}
