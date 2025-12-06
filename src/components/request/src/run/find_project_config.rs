use crate::{Error, FindProjectConfig, Like, Pf, Rt, Run, Suspend, Th, TopErrors, UnwrapSt, log};
use std::sync::Arc;
use vfs::{BlockingFs, Canonical};

impl<'e, P: Pf> Run<'e, P> for FindProjectConfig {
    fn run(&self, st: &mut P::St<'e>, th: &mut impl Th<'e, P>) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());

        let filename = self.working_directory.join("adept.build");

        let Ok(path) = Canonical::new(&filename) else {
            return Ok(Err(TopErrors::new_one(Error::FailedToCanonicalize(
                Arc::from(filename.into_boxed_path()),
            ))));
        };

        match th.rt().vfs().read::<BlockingFs>(path.clone()) {
            Ok(got) => {
                if got.changed_at.is_some() {
                    log!("  New content for adept.build is: {:?}", got.content.text());
                }

                let Ok(text) = got.content.text() else {
                    return Ok(Err(TopErrors::new_one(Error::FailedToOpenFile(Arc::new(
                        path,
                    )))));
                };

                Ok(Ok(text))
            }
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => Error::MissingProjectFile.into(),
                _ => Error::FailedToOpenProjectFile.into(),
            },
        }
    }
}
