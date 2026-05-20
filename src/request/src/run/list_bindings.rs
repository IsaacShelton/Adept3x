use crate::{Like, ListBindings, Pf, Run, Suspend, Th, UnwrapSt};
use with_errors::WithErrors;

impl<'e, P: Pf> Run<'e, P> for ListBindings {
    fn run(
        &self,
        _aft: Option<&Self::Aft<'e>>,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());

        let parsed = th.demand(crate::ParseFile {
            filename: self.filename.clone(),
        })?;

        let names = parsed.value.as_ref().into_iter().flat_map(|parsed| {
            parsed
                .bindings()
                .flat_map(|binding| binding.name.as_ref().map(|s| s.to_string()))
        });

        Ok(WithErrors::new(names.collect(), parsed.errors.clone()))
    }
}
