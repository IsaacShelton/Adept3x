use crate::{GetAst, Like, Pf, Run, Suspend, Th, UnwrapSt};
use std::{path::PathBuf, sync::Arc};
use vfs::Canonical;

pub struct Source {
    path: Arc<Canonical<PathBuf>>,
}

impl<'e, P: Pf> Run<'e, P> for GetAst {
    fn run(&self, st: &mut P::St<'e>, th: &mut impl Th<'e, P>) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());
        todo!("get ast")
    }
}
