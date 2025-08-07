use crate::{
    Continuation, Executable, ExecutionCtx, Executor, execution::main::read_file::ReadFile,
    module_graph::ModulePartHandle, sub_task::SubTask,
};
use build_ast::{Input, Parser};
use build_token::Lexer;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use infinite_iterator::InfiniteIteratorPeeker;
use primitives::CIntegerAssumptions;
use source_files::SourceFiles;
use std::{fs::canonicalize, path::PathBuf};
use text::{CharacterInfiniteIterator, CharacterPeeker};

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LoadFile<'env> {
    canonical_filename: Result<PathBuf, PathBuf>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    read_file: ReadFile,

    module_part_handle: ModulePartHandle<'env>,
}

impl<'env> LoadFile<'env> {
    pub fn new(filename: PathBuf, module_part_handle: ModulePartHandle<'env>) -> Self {
        Self {
            // TODO: Better handle canonicalization
            canonical_filename: canonicalize(&filename).map_err(|_| filename.clone()),
            read_file: ReadFile::new(filename),
            module_part_handle,
        }
    }
}

impl<'env> Executable<'env> for LoadFile<'env> {
    // The filepath to execute when completed
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        // Ensure filename was canonicalized
        let canonical_filename = self.canonical_filename.as_ref().map_err(|filename| {
            ErrorDiagnostic::plain(format!(
                "Failed to canonicalize filename: {}",
                filename.to_string_lossy()
            ))
        })?;

        // Read file content
        let content = execute_sub_task!(self, self.read_file, executor, ctx)
            .map_err(ErrorDiagnostic::plain)?;

        // TODO: We will need to migrate `Compiler` data to keep things like `SourceFiles`
        let source_files = &SourceFiles::new();
        let key = source_files.add(canonical_filename.into(), content.into());
        let content = source_files.get(key).content();

        println!("got content {:?}", content);

        let text = CharacterPeeker::new(CharacterInfiniteIterator::new(content.chars(), key));
        let lexer = InfiniteIteratorPeeker::new(Lexer::new(text));
        let mut input = Input::new(lexer, source_files, key);
        input.ignore_newlines();

        // TODO: Parse header here...
        input.ignore_newlines();

        let ast = Parser::new(
            input,
            ast::ConformBehavior::Adept(CIntegerAssumptions::default()),
        )
        .parse();

        println!("{:?}", ast);
        Ok(())
    }
}
