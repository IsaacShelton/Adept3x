use crate::{Approach, GetProject, Like, ListSymbols, Pf, Run, Suspend, Th, UnwrapSt, try_ok};
use std::sync::Arc;

impl<'e, P: Pf> Run<'e, P> for ListSymbols {
    fn run(
        &self,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend<'e, P>> {
        let _st = Self::unwrap_st(st.like_mut());

        let project = try_ok!(
            th.demand(GetProject {
                working_directory: Arc::from(
                    std::env::current_dir()
                        .expect("Failed to get current working directory")
                        .into_boxed_path(),
                ),
            })?
        );

        let syms = th.demand(Approach)?;

        let mut list = vec![];
        for (name, _) in syms.named.iter() {
            list.push(name.into());
        }

        Ok(Ok(list))
    }
}
