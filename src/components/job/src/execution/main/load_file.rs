use crate::{
    Continuation, Executable, ExecutionCtx, Executor,
    execution::{main::read_file::ReadFile, resolve::ResolveNamespaceItems},
    module_graph::ModuleView,
    repr::Compiler,
    sub_task::SubTask,
};
use build_ast::{Input, Parser};
use build_token::Lexer;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use infinite_iterator::InfiniteIteratorPeeker;
use primitives::CIntegerAssumptions;
use source_files::Source;
use std::path::Path;
use text::{CharacterInfiniteIterator, CharacterPeeker};

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LoadFile<'env> {
    canonical_filename: &'env Path,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    compiler: &'env Compiler<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    read_file: ReadFile<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    source: Option<Source>,

    view: ModuleView<'env>,
}

impl<'env> LoadFile<'env> {
    pub fn new(
        compiler: &'env Compiler,
        canonical_filename: &'env Path,
        view: ModuleView<'env>,
        source: Option<Source>,
    ) -> Self {
        Self {
            compiler,
            read_file: ReadFile::new(canonical_filename),
            view,
            source,
            canonical_filename,
        }
    }
}

impl<'env> Executable<'env> for LoadFile<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        // Read file content
        let content = execute_sub_task!(self, self.read_file, executor, ctx)
            .map_err(ErrorDiagnostic::plain)?;

        // TODO: We will need to migrate `Compiler` data to keep things like `SourceFiles`
        let source_files = &self.compiler.source_files;
        let key = source_files.add(self.canonical_filename.into(), content.into());
        let content = source_files.get(key).content();

        let text = CharacterPeeker::new(CharacterInfiniteIterator::new(content.chars(), key));
        let lexer = InfiniteIteratorPeeker::new(Lexer::new(text));
        let input = Input::new(lexer, source_files, key);

        let mut parser = Parser::new(
            input,
            ast::ConformBehavior::Adept(CIntegerAssumptions::default()),
        );
        let _file_header = parser.parse_file_header()?;

        let ast = parser.parse().map_err(ErrorDiagnostic::from)?;

        /*
        writeln!(
            &mut std::io::stdout(),
            "[{}]: {:?}",
            self.view.graph(|graph| graph.meta().title),
            self.compiler.filename(&self.canonical_filename),
        )
        .unwrap();
        */

        let _ = executor.spawn_raw(ResolveNamespaceItems::new(
            self.view,
            &self.compiler,
            ctx.alloc(ast),
        ));

        Ok(())
    }
}
