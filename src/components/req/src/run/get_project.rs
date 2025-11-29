use crate::{
    Err, Errs, FindProjectConfig, GetProject, Like, Pf, Project, Run, Suspend, Th, UnwrapSt,
};
use build_token::Lexer;
use infinite_iterator::{InfiniteIteratorPeeker, InfinitePeekable};
use std::{path::PathBuf, sync::Arc};
use text::{CharacterInfiniteIterator, CharacterPeeker};

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

        let chars =
            CharacterPeeker::new(CharacterInfiniteIterator::new(content.chars(), |loc| loc));
        let mut lexer = InfiniteIteratorPeeker::new(Lexer::new(chars));

        if !lexer.peek().is_open_curly() {
            return Ok(Err(Errs::from(Err::ExpectedChar('{')).into()));
        }

        const _VERSION: &'static str = env!("CARGO_PKG_VERSION");
        let path = PathBuf::from("");

        Ok(Ok(Arc::new(Project {
            root: Arc::from(path.into_boxed_path()),
        })))
    }
}
