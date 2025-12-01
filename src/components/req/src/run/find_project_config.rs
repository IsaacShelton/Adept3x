use crate::{Error, FindProjectConfig, Like, Pf, Rt, Run, Suspend, Th, UnwrapSt, log};
use std::io::Read;
use vfs::{BlockingFs, Canonical};

impl<'e, P: Pf> Run<'e, P> for FindProjectConfig {
    fn run(&self, st: &mut P::St<'e>, th: &mut impl Th<'e, P>) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());

        if let Ok(path) = Canonical::new(self.working_directory.join("adept.build")) {
            if let Ok(got) = th.rt().vfs().read::<BlockingFs>(path) {
                if got.changed_at.is_some() {
                    log!("  New content for adept.build is: {:?}", got.content.text());
                }
            }
        }

        match std::fs::File::open(self.working_directory.join("adept.build")) {
            Ok(mut file) => {
                let mut content = String::new();
                match file.read_to_string(&mut content) {
                    Ok(_) => Ok(Ok(content.into())),
                    Err(_) => Error::MissingProjectFile.into(),
                }
            }
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => Error::MissingProjectFile.into(),
                _ => Error::FailedToOpenProjectFile.into(),
            },
        }
    }
}
