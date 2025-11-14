use crate::{
    Artifact, CacheDb, CacheDbString, Control, MustSuspend, NumberedRevision, Req, ReqInfo,
    ReqState, Rt, Thrd, WakeAfter,
};
use diagnostics::ErrorDiagnostic;
use std::{collections::HashMap, time::Duration};
use std_ext::{SmallVec1, SmallVec4};

pub struct IncrRt<'db> {
    requests: HashMap<Req, ReqInfo<NumberedRevision>>,
    when_done_wake: HashMap<Req, SmallVec4<Req>>,
    cache_db: &'db CacheDb,
    errors: HashMap<Req, SmallVec1<ErrorDiagnostic>>,
    current_revision: NumberedRevision,
}

impl<'db> IncrRt<'db> {
    pub fn new(db: &'db CacheDb) -> Self {
        Self {
            requests: Default::default(),
            when_done_wake: Default::default(),
            cache_db: db,
            errors: Default::default(),
            current_revision: NumberedRevision::default(),
        }
    }
}

impl<'db> Rt for IncrRt<'db> {
    type StringKey = CacheDbString;
    type Revision = NumberedRevision;

    fn set_input(&mut self, req: Req, rev: NumberedRevision, value: Artifact) {
        self.requests.insert(req, ReqInfo::complete(rev, value));
        self.current_revision = rev;
    }

    fn block_on(&mut self, req: Req) -> Result<Option<&Artifact>, ErrorDiagnostic> {
        todo!("block_on")
    }

    fn read_str<F, Ret>(&self, string_key: Self::StringKey, f: F) -> Ret
    where
        F: FnMut(&str) -> Ret,
    {
        self.cache_db.read_str(string_key, f)
    }
}

struct IncrThrd<'db, 'rt> {
    runtime: &'rt IncrRt<'db>,
    wake_after: Vec<Req>,
}

impl<'db, 'rt> Thrd for IncrThrd<'db, 'rt> {
    type Runtime = IncrRt<'db>;

    fn alloc_str(&mut self, content: &str) -> <Self::Runtime as Rt>::StringKey {
        todo!()
    }

    fn runtime(&self) -> &Self::Runtime {
        self.runtime
    }

    fn demand(&mut self, req: Req) -> Result<&Artifact, MustSuspend> {
        todo!()
    }
}
