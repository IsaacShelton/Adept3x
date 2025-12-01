use crate::{Approach, GetProject, Like, Pf, Run, Suspend, Syms, Th, UnwrapSt, WithErrors};
use std::sync::Arc;

impl<'e, P: Pf> Run<'e, P> for Approach {
    fn run(&self, st: &mut P::St<'e>, th: &mut impl Th<'e, P>) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());
        let new_syms = Syms::default();

        let project = match th.demand(GetProject {
            working_directory: Arc::from(
                std::env::current_dir()
                    .expect("Failed to get current working directory")
                    .into_boxed_path(),
            ),
        })? {
            Ok(project) => project,
            Err(errors) => return Ok(WithErrors::new(new_syms, errors.clone())),
        };

        // Warning: we don't actually approach yet...
        Ok(WithErrors::no_errors(new_syms))
    }
}
