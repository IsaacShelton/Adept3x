use super::super::syntax_tree::GreenKind;
use crate::{
    Error, GetProject, GetSyntaxTree, Like, Pf, Rt, Run, Suspend, SymGrp, Symbols, Syms, Th,
    TopErrors, UnwrapSt, WithErrors,
};
use std::sync::Arc;
use vfs::Canonical;

impl<'e, P: Pf> Run<'e, P> for Symbols {
    fn run(
        &self,
        _aft: Option<&Self::Aft<'e>>,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());
        let mut new_syms = Syms::default();

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

        let WithErrors {
            value: syntax_tree,
            errors,
        } = th.demand(GetSyntaxTree {
            filename: Arc::new(filename),
        })?;

        // If there were errors getting the syntax tree, exit.
        // This will only really happen if we failed to read the file.
        if !errors.is_empty() {
            return Ok(WithErrors::new(new_syms, errors.clone()));
        }

        let red_tree = syntax_tree.red_tree();

        if let GreenKind::Value = red_tree.green.kind {
            if let Some(array) = red_tree
                .children()
                .filter(|node| node.green.kind.is_array())
                .next()
            {
                for value in array.children().filter(|node| node.green.kind.is_value()) {
                    for string in value.children().filter(|node| node.green.kind.is_string()) {
                        if let Some(text) = &string.green.text {
                            let grp = SymGrp::new_empty(th.rt().current());
                            new_syms.named.insert(text.into(), grp);
                        }
                    }
                }
            }
        }

        // Warning: we don't actually approach yet...
        Ok(WithErrors::no_errors(new_syms))
    }
}
