use super::Executable;
use crate::{Continuation, ExecutionCtx, Executor};
use compiler::BuildOptions;
use diagnostics::ErrorDiagnostic;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Main<'env> {
    #[allow(unused)]
    build_options: &'env BuildOptions,

    #[allow(unused)]
    project_folder: &'env Path,

    #[allow(unused)]
    single_file: Option<&'env Path>,
}

impl<'env> Main<'env> {
    pub fn new(
        build_options: &'env BuildOptions,
        project_folder: &'env Path,
        single_file: Option<&'env Path>,
    ) -> Self {
        Self {
            build_options,
            project_folder,
            single_file,
        }
    }
}

impl<'env> Executable<'env> for Main<'env> {
    // The filepath to execute when completed
    type Output = Option<&'env Path>;

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let Some(single_file) = self.single_file else {
            return Err(ErrorDiagnostic::plain("Must specify root file").into());
        };

        println!("{:?}", single_file);
        Ok(None)
    }
}
