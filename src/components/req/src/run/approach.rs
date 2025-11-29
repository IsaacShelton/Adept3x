use crate::{Approach, Like, Pf, Run, Suspend, Syms, Th, UnwrapSt};

impl<'e, P: Pf> Run<'e, P> for Approach {
    fn run(
        &self,
        st: &mut P::St<'e>,
        _th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend<'e, P>> {
        let _st = Self::unwrap_st(st.like_mut());
        let new_syms = Syms::default();
        // Warning: we don't actually approach yet...
        Ok(new_syms)
    }
}
