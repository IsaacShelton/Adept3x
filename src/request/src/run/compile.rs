use crate::{Compile, Like, Pf, Run, Suspend, Th, UnwrapSt};
use std::sync::Arc;
use with_errors::WithErrors;

impl<'e, P: Pf> Run<'e, P> for Compile {
    fn run(
        &self,
        _aft: Option<&Self::Aft<'e>>,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<Self::Aft<'e>, Suspend> {
        let _st = Self::unwrap_st(st.like_mut());

        let parsed = th
            .demand(crate::ParseFile {
                filename: self.filename.clone(),
            })?
            .clone();

        let result = th
            .demand(crate::ListBindings {
                filename: self.filename.clone(),
            })?
            .clone();

        if let Some(parsed) = &parsed.value {
            let bindings = parsed.0.bindings();
            let symbols = kernel::elaborate_symbols(bindings);
            log::info!("symbols is {:#?}", symbols);

            if !symbols.errors.is_empty() {
                return Ok(WithErrors::new(Arc::new([]), symbols.errors));
            }

            return Ok(kernel::compile_executable(&symbols.value).map(|_| Arc::from([])));
        }

        Ok(result)
    }
}
