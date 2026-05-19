use crate::{Compile, Like, Pf, Run, Suspend, Th, UnwrapSt};
use std::sync::Arc;
use with_errors::{Error, WithErrors};

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

            let symbols = symbols.value;

            let main = symbols.lookup("main");

            let Some(main) = main else {
                return Ok(WithErrors::new_one(
                    Arc::new([]),
                    Error::MissingMainFunction,
                ));
            };

            match main.ty.is_main_function_ty() {
                Ok(true) => (),
                Ok(false) => {
                    return Ok(WithErrors::new_one(
                        Arc::new([]),
                        Error::IncorrectSignatureForMainFunction,
                    ));
                }
                Err(message) => {
                    return Ok(WithErrors::new_one(
                        Arc::new([]),
                        Error::TypingError(message),
                    ));
                }
            }
        }

        Ok(result)
    }
}
