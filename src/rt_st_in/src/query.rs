use crate::{Pf, QueryThen};
use connection::Connection;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

pub struct RtStInQuery<'e, P: Pf> {
    pub(crate) queue: Vec<P::Req<'e>>,
    pub(crate) waiting: HashMap<P::Req<'e>, Vec<P::Req<'e>>>,
    pub(crate) rev: P::Rev,
    pub(crate) req: P::Req<'e>,
    pub(crate) files: QueryFiles,
    pub then: QueryThen<'e, P>,
    pub connection: Connection,
}

pub type QueryFiles = HashMap<PathBuf, Arc<str>>;
