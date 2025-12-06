use crate::{
    Error, GetAst, GetProject, Like, Pf, Run, Suspend, Symbols, Syms, Th, TopErrors, UnwrapSt,
    WithErrors,
};
use std::sync::Arc;
use vfs::Canonical;

impl<'e, P: Pf> Run<'e, P> for Symbols {
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

        let filename = match Canonical::new(&project.root) {
            Ok(filename) => filename,
            Err(()) => {
                return Ok(WithErrors::new(
                    Default::default(),
                    TopErrors::new_one(Error::FailedToCanonicalize(project.root.clone())),
                ));
            }
        };

        let _ = th.demand(GetAst {
            filename: Arc::new(filename),
        });

        // Warning: we don't actually approach yet...
        Ok(WithErrors::no_errors(new_syms))
    }
}
