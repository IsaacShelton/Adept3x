use crate::{GetRootSourceFile, Like, Pf, Run, Suspend, Th, UnwrapSt};

impl<'e, P: Pf> Run<'e, P> for GetRootSourceFile {
    fn run(
        &self,
        st: &mut P::St<'e>,
        _th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend<'e, P>> {
        let _st = Self::unwrap_st(st.like_mut());
        todo!("get root source file")
    }
}
