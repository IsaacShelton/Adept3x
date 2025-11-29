use crate::{Pf, Syms};
use std::collections::HashMap;

pub struct RtStInQuery<'e, P: Pf> {
    pub(crate) queue: Vec<P::Req<'e>>,
    pub(crate) waiting: HashMap<P::Req<'e>, Vec<P::Req<'e>>>,
    pub(crate) rev: P::Rev,
    pub(crate) req: P::Req<'e>,
    pub(crate) new_syms: Option<Syms<P>>,
}
