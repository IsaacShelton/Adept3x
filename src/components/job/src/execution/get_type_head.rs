use super::Executable;
use crate::{Continuation, ExecutionCtx, Executor, repr::TypeHead};
use ast_workspace::{AstWorkspace, TypeDeclRef};
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GetTypeHead<'env> {
    workspace: ByAddress<&'env AstWorkspace<'env>>,
    type_decl_ref: TypeDeclRef,
}

impl<'env> GetTypeHead<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>, type_decl_ref: TypeDeclRef) -> Self {
        Self {
            workspace: ByAddress(workspace),
            type_decl_ref,
        }
    }
}

impl<'env> Executable<'env> for GetTypeHead<'env> {
    type Output = &'env TypeHead<'env>;

    fn execute(
        self,
        _executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;
        let symbols = &workspace.symbols;

        let (name, arity) = match self.type_decl_ref {
            TypeDeclRef::Struct(idx) => {
                let def = &symbols.all_structs[idx];
                (&def.name, def.params.len())
            }
            TypeDeclRef::Enum(idx) => {
                let def = &symbols.all_enums[idx];
                (&def.name, 0)
            }
            TypeDeclRef::Alias(idx) => {
                let def = &symbols.all_type_aliases[idx];
                (&def.name, def.params.len())
            }
            TypeDeclRef::Trait(idx) => {
                let def = &symbols.all_traits[idx];
                (&def.name, def.params.len())
            }
        };

        Ok(ctx.alloc(TypeHead::new(name, arity)))
    }
}
