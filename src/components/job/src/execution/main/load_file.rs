use crate::{
    Continuation, Executable, ExecutionCtx, Executor,
    execution::main::read_file::ReadFile,
    module_graph::{ModuleBreakOffMode, ModuleView},
    repr::Compiler,
    sub_task::SubTask,
};
use build_ast::{Input, Parser};
use build_token::Lexer;
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use infinite_iterator::InfiniteIteratorPeeker;
use primitives::CIntegerAssumptions;
use source_files::Source;
use std::{
    fs::canonicalize,
    path::{Path, PathBuf},
};
use text::{CharacterInfiniteIterator, CharacterPeeker};

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LoadFile<'env> {
    canonical_filename: Result<PathBuf, PathBuf>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    read_file: ReadFile,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    source: Option<Source>,

    view: ModuleView<'env>,
}

impl<'env> LoadFile<'env> {
    pub fn new(
        compiler: &'env Compiler,
        filename: PathBuf,
        view: ModuleView<'env>,
        source: Option<Source>,
    ) -> Self {
        Self {
            canonical_filename: canonicalize(&filename).map_err(|_| filename.clone()),
            compiler: ByAddress(compiler),
            read_file: ReadFile::new(filename),
            view,
            source,
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
        // Ensure filename was canonicalized
        let canonical_filename = self.canonical_filename.as_ref().map_err(|filename| {
            if let Ok(false) = std::fs::exists(filename) {
                ErrorDiagnostic::new_maybe_source(
                    format!("File does not exist: {}", filename.to_string_lossy()),
                    self.source,
                )
            } else {
                ErrorDiagnostic::new_maybe_source(
                    format!(
                        "Failed to canonicalize filename: {}",
                        filename.to_string_lossy()
                    ),
                    self.source,
                )
            }
        })?;

        // Read file content
        let content = execute_sub_task!(self, self.read_file, executor, ctx)
            .map_err(ErrorDiagnostic::plain)?;

        // TODO: We will need to migrate `Compiler` data to keep things like `SourceFiles`
        let source_files = &self.compiler.source_files;
        let key = source_files.add(canonical_filename.into(), content.into());
        let content = source_files.get(key).content();

        let text = CharacterPeeker::new(CharacterInfiniteIterator::new(content.chars(), key));
        let lexer = InfiniteIteratorPeeker::new(Lexer::new(text));
        let input = Input::new(lexer, source_files, key);

        let mut parser = Parser::new(
            input,
            ast::ConformBehavior::Adept(CIntegerAssumptions::default()),
        );

        let _file_header = parser.parse_file_header()?;
        /*
        println!(
            "Adept: {}",
            file_header
                .as_ref()
                .into_iter()
                .flat_map(|x| x.adept.as_ref())
                .map(|x| x.to_string())
                .next()
                .unwrap_or("Default".into())
        );
        */

        let ast = parser.parse().map_err(ErrorDiagnostic::from)?;
        println!("{:?}", canonical_filename);

        for namespace in ast.namespaces {
            match namespace.items {
                ast::NamespaceItemsSource::Items(_namespace_items) => {
                    todo!("namespace items not supported yet")
                }
                ast::NamespaceItemsSource::Expr(expr) => {
                    let Some(load_target) = fake_run_namespace_expr(&expr) else {
                        return Err(ErrorDiagnostic::new(
                            "Expression must evaluate to a target to import or incorporate",
                            expr.source,
                        )
                        .into());
                    };

                    let new_filename = canonical_filename
                        .parent()
                        .expect("file is in folder")
                        .join(Path::new(&load_target.relative_filename));

                    let new_view = self.view.break_off(load_target.mode);
                    let _ = executor.request(LoadFile::new(
                        &self.compiler,
                        new_filename,
                        new_view,
                        Some(expr.source),
                    ));
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct LoadTarget {
    mode: ModuleBreakOffMode,
    relative_filename: String,
}

// Eventually, we'll hook this up to the comptime VM for a more powerful version.
// We'll keep it simple for now though.
fn fake_run_namespace_expr(expr: &ast::Expr) -> Option<LoadTarget> {
    let ast::ExprKind::Call(call) = &expr.kind else {
        return None;
    };

    let mode = match call.name.as_plain_str() {
        Some("incorporate") => ModuleBreakOffMode::Part,
        Some("import") => ModuleBreakOffMode::Module,
        _ => return None,
    };

    if call.args.len() != 1 {
        return None;
    }

    let ast::ExprKind::String(filename) = &call.args[0].kind else {
        return None;
    };

    Some(LoadTarget {
        mode,
        relative_filename: filename.into(),
    })
}
