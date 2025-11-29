use crate::{Err, Errs, FindProjectConfig, Like, Pf, Rt, Run, Suspend, Th, UnwrapSt};
use std::io::Read;
use vfs::{BlockingFs, Canonical};

impl<'e, P: Pf> Run<'e, P> for FindProjectConfig {
    fn run(
        &self,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend<'e, P>> {
        let _st = Self::unwrap_st(st.like_mut());

        if let Ok(path) = Canonical::new(self.working_directory.join("adept.build")) {
            if let Ok(got) = th.rt().vfs().read::<BlockingFs>(path) {
                if got.changed_at.is_some() {
                    let _ = dbg!(got.content.text());
                }
            }
        }

        Ok(
            match std::fs::File::open(self.working_directory.join("adept.build")) {
                Ok(mut file) => {
                    let mut content = String::new();
                    match file.read_to_string(&mut content) {
                        Ok(_) => Ok(content.into()),
                        Err(_) => Err(Errs::from(Err::MissingProjectFile).into()),
                    }
                }
                Err(error) => match error.kind() {
                    std::io::ErrorKind::NotFound => Err(Errs::from(Err::MissingProjectFile).into()),
                    _ => Err(Errs::from(Err::FailedToOpenProjectFile).into()),
                },
            },
        )
    }
}
