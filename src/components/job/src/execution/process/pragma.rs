use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ProcessFile, canonicalize_or_error,
    module_graph::{ModuleBreakOffMode, ModuleView, Upserted},
    repr::{Compiler, DeclHead, DeclHeadTypeLike},
};
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use std::path::Path;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ProcessPragma<'env> {
    view: &'env ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    pragma: ByAddress<&'env ast::Pragma>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    load_target: Option<LoadTarget>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    spawned_children: bool,
}

impl<'env> ProcessPragma<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        pragma: &'env ast::Pragma,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            pragma: ByAddress(pragma),
            load_target: None,
            spawned_children: false,
        }
    }
}

impl<'env> Executable<'env> for ProcessPragma<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        if self.spawned_children {
            return Ok(());
        }

        let Some(load_target) = fake_run_namespace_expr(&self.pragma.expr) else {
            return Err(ErrorDiagnostic::new(
                "Expression must evaluate to a target to import or add",
                self.pragma.expr.source,
            )
            .into());
        };

        let new_filename = ctx.alloc(canonicalize_or_error(
            Some(&self.compiler),
            &self
                .view
                .canonical_filename
                .parent()
                .expect("file is in folder")
                .join(Path::new(&load_target.relative_filename)),
            Some(self.pragma.expr.source),
            self.view.graph,
        )?);

        let new_view = self
            .view
            .break_off(load_target.mode, new_filename, &self.compiler);

        self.spawned_children = true;

        let Upserted::Created(created) = new_view else {
            return Ok(());
        };

        if let Some(use_binding) = &self.pragma.name {
            match use_binding {
                ast::UseBinding::Name(name) => {
                    self.view.add_symbol(
                        self.pragma.0.privacy,
                        name,
                        DeclHead::TypeLike(DeclHeadTypeLike::Namespace(
                            name,
                            created.handle.module_ref,
                        )),
                    );
                }
                ast::UseBinding::Wildcard => todo!("use binding wildcard not implemented yet!"),
            }
        }

        ctx.suspend_on(std::iter::once(
            executor
                .request(ProcessFile::new(
                    &self.compiler,
                    new_filename,
                    ctx.alloc(created),
                    Some(self.pragma.expr.source),
                ))
                .raw_task_ref(),
        ));
        return Err(Continuation::Suspend(self.into()));
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

    let mode = match call.name_path.as_plain_str() {
        Some("include") => ModuleBreakOffMode::Part,
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
