use crate::{Pf, Run, Suspend, Th, UnusedRequest};

impl<'e, P: Pf> Run<'e, P> for UnusedRequest {
    fn run(
        &self,
        _aft: Option<&Self::Aft<'e>>,
        _st: &mut P::St<'e>,
        _th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend> {
        unreachable!();
    }
}
