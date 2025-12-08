use crate::{Like, Pf, Run, Search, Suspend, Th, UnwrapSt};

impl<'e, P: Pf> Run<'e, P> for Search {
    fn run(
        &self,
        _aft: Option<&Self::Aft<'e>>,
        st: &mut P::St<'e>,
        _th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());
        todo!()
    }
}
