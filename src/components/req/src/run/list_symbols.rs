use crate::{GetProject, Like, ListSymbols, Pf, Run, Suspend, Th, UnwrapSt};
use std::sync::Arc;

impl<'e, P: Pf> Run<'e, P> for ListSymbols {
    fn run(
        &self,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend<'e, P>> {
        let _st = Self::unwrap_st(st.like_mut());

        let project = th.demand(GetProject {
            working_directory: Arc::from(
                std::env::current_dir()
                    .expect("Failed to get current working directory")
                    .into_boxed_path(),
            ),
        })?;

        Ok(vec![format!("{:?}", project)])
    }
}
