use crate::{Like, ListSymbols, Pf, Run, Suspend, Symbols, Th, UnwrapSt, WithErrors};

impl<'e, P: Pf> Run<'e, P> for ListSymbols {
    fn run(&self, st: &mut P::St<'e>, th: &mut impl Th<'e, P>) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());

        let WithErrors {
            value: syms,
            errors,
        } = th.demand(Symbols)?;

        let mut list = vec![
            "example".into(),
            "another_example".into(),
            "final_example".into(),
        ];
        for (name, _) in syms.named.iter() {
            list.push(name.into());
        }

        Ok(WithErrors::new(list, errors.clone()))
    }
}
