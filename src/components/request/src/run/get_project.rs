use crate::{
    Error, FindProjectConfig, GetProject, Like, Pf, Project, Run, Suspend, Th, TopErrorsNode,
    UnwrapSt,
};
use build_aon::{Aon, parse_aon};
use build_token::Lexer;
use infinite_iterator::InfiniteIteratorPeeker;
use std::{path::PathBuf, sync::Arc};
use text::{CharacterInfiniteIterator, CharacterPeeker};

impl<'e, P: Pf> Run<'e, P> for GetProject {
    fn run(&self, st: &mut P::St<'e>, th: &mut impl Th<'e, P>) -> Result<Self::Aft<'e>, Suspend> {
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

        let Ok(mut config) = parse_aon(&mut lexer) else {
            return Error::InvalidProjectConfigSyntax.into();
        };

        if config
            .remove("adept")
            .and_then(|v| v.into_string())
            .as_ref()
            .map(|s| s.as_str())
            != Some("3.0")
        {
            return Error::UnsupportedAdeptVersion.into();
        };

        let Some(main) = config.remove("main").and_then(|entry| entry.into_string()) else {
            return Error::MissingRootFileInProjectConfig.into();
        };

        let interval_ms = config
            .remove("interval_ms")
            .and_then(|value| value.into_u64());

        let max_idle_time_ms = config
            .remove("max_idle_time_ms")
            .and_then(|value| value.into_u64());

        let cache_to_disk = config
            .remove("cache_to_disk")
            .and_then(|value| value.into_bool());

        let Aon::Object(remaining) = config else {
            return Error::InvalidProjectConfigSyntax.into();
        };

        let errors = TopErrorsNode::from_iter(
            remaining
                .into_keys()
                .map(|name| Error::InvalidProjectConfigOption(Arc::from(name))),
        );

        if !errors.is_empty() {
            return Ok(Err(errors.into()));
        }

        Ok(Ok(Project {
            root: Arc::from(PathBuf::from(main).into_boxed_path()),
            interval_ms,
            max_idle_time_ms,
            cache_to_disk,
        }))
    }
}
