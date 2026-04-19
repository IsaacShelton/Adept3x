use crate::{Error, Like, ParseFile, Pf, Run, Suspend, Th, UnwrapSt, WithErrors};
use by_address::ByAddress;
use document::Document;

impl<'e, P: Pf> Run<'e, P> for ParseFile {
    fn run(
        &self,
        _aft: Option<&Self::Aft<'e>>,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());

        let content = th.read_file(&self.filename);

        let Ok(content) = &content else {
            return Ok(WithErrors::new_one(
                None,
                Error::FailedToOpenFile(self.filename.clone()),
            ));
        };

        let document = Document::new(content);
        let syntax_tree = parser_adept::reparse(&document, None, document.full_range());
        // let _ = syntax_tree.dump(&mut std::io::stdout(), 0);
        Ok(WithErrors::no_errors(Some(ByAddress(syntax_tree))))
    }
}
