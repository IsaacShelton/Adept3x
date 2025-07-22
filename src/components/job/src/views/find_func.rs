use super::{FindFailed, FuncLikeSearch, ViewRef, ViewsInner};
use crate::{
    BuiltinTypes, PolyCatalog, PolyRecipe, PolyValue,
    conform::conform_to_default,
    repr::{FuncHead, Type},
};
use asg::FuncId;
use ast_workspace::FuncRef;
use primitives::CIntegerAssumptions;
use source_files::Source;
use std::collections::HashSet;
use std_ext::SmallVec4;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Callee<'env> {
    pub func_ref: FuncRef,
    pub recipe: PolyRecipe<'env>,
}

impl<'env, 'syms> ViewsInner<'env, 'syms> {
    pub fn try_find_func_like(
        &self,
        starting_view_ref: ViewRef,
        name: &str,
        search: &FuncLikeSearch,
    ) -> Result<Callee<'env>, FindFailed> {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut found = SmallVec4::new();

        stack.push(starting_view_ref.into_raw());
        visited.insert(starting_view_ref.into_raw());

        while let Some(view_id) = stack.pop() {
            let view_ref = unsafe { ViewRef::from_raw(view_id) };
            for parent in self.inclusion.parents(view_ref) {
                if visited.insert(parent.into_raw()) {
                    stack.push(parent.into_raw());
                }
            }

            let view = &self.views[view_ref];
            let Some(decls) = view.symbols.get(name) else {
                continue;
            };

            for (func_like, func_head) in decls.func_likes() {
                let matches = func_head.params.required.len() == search.args.len()
                    || (func_head.params.is_cstyle_vararg
                        && func_head.params.required.len() < search.args.len());

                if matches {
                    found.push(func_like);
                }
            }
        }

        Err(FindFailed::NotFound)
    }
}
