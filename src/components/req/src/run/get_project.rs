use crate::{
    Err, Errs, FindProjectConfig, GetProject, Like, Pf, Project, Run, Suspend, Th, UnwrapSt,
};
use std::{path::PathBuf, sync::Arc};
use text::{CharacterInfiniteIterator, CharacterPeeker, Text};

impl<'e, P: Pf> Run<'e, P> for GetProject {
    fn run(
        &self,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend<'e, P>> {
        let _st = Self::unwrap_st(st.like_mut());

        let content = match th.demand(FindProjectConfig {
            working_directory: self.working_directory.clone(),
        })? {
            Ok(content) => content,
            Err(errors) => return Ok(Err(errors.clone())),
        };

        let mut chars = CharacterPeeker::new(CharacterInfiniteIterator::new(content.chars()));

        if !chars.eat('{') {
            return Ok(Err(Errs::from(Err::ExpectedChar('{')).into()));
        }

        const _VERSION: &'static str = env!("CARGO_PKG_VERSION");
        let path = PathBuf::from("");

        Ok(Ok(Arc::new(Project {
            root: Arc::from(path.into_boxed_path()),
        })))
    }
}
