use crate::{
    Error, GetSyntaxTree, Like, Pf, Rt, Run, Suspend, Th, UnwrapSt, WithErrors,
    syntax_tree::SyntaxTree,
};
use vfs::BlockingFs;

impl<'e, P: Pf> Run<'e, P> for GetSyntaxTree {
    fn run(
        &self,
        aft: Option<&Self::Aft<'e>>,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());

        let content = match th.rt().vfs().read::<BlockingFs>(self.filename.clone()) {
            Ok(content) => match content.content.text() {
                Ok(text) => text,
                Err(_) => {
                    return Ok(WithErrors::new_one(
                        SyntaxTree::default(),
                        Error::FailedToOpenFile(self.filename.clone()),
                    ));
                }
            },
            Err(_) => {
                return Ok(WithErrors::new_one(
                    SyntaxTree::default(),
                    Error::FailedToOpenFile(self.filename.clone()),
                ));
            }
        };

        Ok(WithErrors::no_errors(SyntaxTree::parse(
            content.clone(),
            Some(self.filename.clone()),
        )))
    }
}
