#![allow(unused)]

mod find_func;

use crate::repr::{DeclHead, DeclHeadSet, Type, TypeLikeRef};
use arena::{Arena, ArenaMap, Idx, LockFreeArena, new_id_with_niche};
use ast_workspace::{AstWorkspaceSymbols, ExprAliasRef, FuncRef, TypeDecl, TypeDeclRef};
use derivative::Derivative;
use source_files::Source;
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    mem::MaybeUninit,
    sync::RwLock,
};
use std_ext::{HashMapExt, SmallVec4, SmallVec8};

new_id_with_niche!(ViewId, u32);
pub type ViewRef<'env> = Idx<ViewId, View<'env>>;

pub struct Views<'env, 'syms> {
    lock: RwLock<ViewsInner<'env, 'syms>>,
}

impl<'env, 'syms> Views<'env, 'syms> {
    pub fn add_symbols(
        &self,
        view_ref: ViewRef<'env>,
        symbols: HashMap<&'env str, DeclHeadSet<'env>>,
    ) -> Result<(), BrokenLinks<'env>> {
        self.lock.write().unwrap().add_symbols(view_ref, symbols)
    }
}

pub struct ViewsInner<'env, 'syms> {
    views: LockFreeArena<ViewId, View<'env>>,
    inclusion: InclusionGraph<'env>,
    all_symbols: &'syms AstWorkspaceSymbols,

    // Reused allocations
    visited: HashSet<ViewId>,
    stack: Vec<ViewId>,
}

impl<'env, 'syms> ViewsInner<'env, 'syms> {
    pub fn add_symbols(
        &mut self,
        starting_view_ref: ViewRef<'env>,
        symbols: HashMap<&'env str, DeclHeadSet<'env>>,
    ) -> Result<(), BrokenLinks<'env>> {
        // Check that all ancestor views don't have any links
        // that change by this addition...
        let mut broken_links = BrokenLinks::default();

        self.visited.clear();
        self.visited.insert(starting_view_ref.into_raw());

        self.stack.clear();
        self.stack.push(starting_view_ref.into_raw());

        while let Some(view_id) = self.stack.pop() {
            // Get reference to view (re-usage of data structures here requires unsafe)
            // SAFETY: We are assuming that our elements are to the arena we're using.
            let view_ref = unsafe { ViewRef::from_raw(view_id) };

            // Enqueue parents
            for parent in self.inclusion.parents(view_ref) {
                if self.visited.insert(parent.into_raw()) {
                    self.stack.push(parent.into_raw());
                }
            }

            let view = &self.views[view_ref];

            for (name, links) in view.links.iter() {
                for type_like in links.type_like_links.iter() {
                    if self
                        .try_find_type_like(view_ref, name, &type_like.search)
                        .is_err()
                    {
                        broken_links
                            .links
                            .entry(*name)
                            .or_default()
                            .type_like_links
                            .push(type_like.clone());
                    }
                }

                for func_like in links.func_like_links.iter() {
                    if self
                        .try_find_func_like(view_ref, name, &func_like.search)
                        .is_err()
                    {
                        broken_links
                            .links
                            .entry(*name)
                            .or_default()
                            .func_like_links
                            .push(func_like.clone());
                    }
                }
            }
        }

        if !broken_links.is_empty() {
            return Err(broken_links);
        }

        // Add symbols to view
        self.views[starting_view_ref]
            .symbols
            .extend(symbols.into_iter());
        Ok(())
    }

    fn try_find_type_like(
        &self,
        starting_view_ref: ViewRef<'env>,
        name: &str,
        search: &TypeLikeSearch,
    ) -> Result<TypeLikeRef, FindFailed> {
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

            for (type_like, type_head) in decls.type_likes() {
                if type_head.arity == search.arity as usize {
                    found.push(type_like);
                }
            }
        }

        Err(FindFailed::NotFound)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindFailed {
    NotFound,
    Ambiguous,
}

#[derive(Clone, Debug, Default)]
pub struct BrokenLinks<'env> {
    links: HashMap<&'env str, Links<'env>>,
}

impl<'env> BrokenLinks<'env> {
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

pub struct InclusionGraph<'env> {
    child_views: ArenaMap<ViewId, Vec<ViewRef<'env>>>,
    parent_views: ArenaMap<ViewId, Vec<ViewRef<'env>>>,
}

impl<'env> InclusionGraph<'env> {
    pub fn children(&self, view_ref: ViewRef<'env>) -> impl Iterator<Item = ViewRef<'env>> {
        self.child_views
            .get(view_ref.into_raw())
            .into_iter()
            .flatten()
            .copied()
    }

    pub fn parents(&self, view_ref: ViewRef<'env>) -> impl Iterator<Item = ViewRef<'env>> {
        self.parent_views
            .get(view_ref.into_raw())
            .into_iter()
            .flatten()
            .copied()
    }
}

type ValueDeclRef = ExprAliasRef;
pub struct View<'env> {
    symbols: HashMap<&'env str, DeclHeadSet<'env>>,
    links: HashMap<&'env str, Links<'env>>,
}

#[derive(Clone, Debug, Default)]
pub struct Links<'env> {
    type_like_links: Vec<TypeLikeLink>,
    func_like_links: Vec<FuncLikeLink<'env>>,
    value_like_links: Vec<ValueLikeLink>,
}

#[derive(Clone, Debug)]
pub struct TypeLikeSearch {
    arity: u32,
}

#[derive(Clone, Debug)]
pub struct TypeLikeLink {
    search: TypeLikeSearch,
    source: Source,
    links_to: TypeLikeRef,
}

#[derive(Clone, Debug)]
pub struct FuncLikeSearch<'env> {
    args: &'env [&'env Type<'env>],
}

#[derive(Clone, Debug)]
pub struct FuncLikeLink<'env> {
    search: FuncLikeSearch<'env>,
    source: Source,
    links_to: FuncRef,
}

#[derive(Clone, Debug)]
pub struct ValueLikeLink {
    source: Source,
    links_to: ValueDeclRef,
}
