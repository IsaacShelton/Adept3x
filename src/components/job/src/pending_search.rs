use crate::{TaskRef, module_graph::ModuleGraphRef};
use std::{
    collections::HashMap,
    sync::{Mutex, RwLock},
};

#[derive(Debug, Default)]
pub struct ModuleGraphPendingSearchMap<'env> {
    inner: RwLock<HashMap<ModuleGraphRef, PendingSearchMap<'env>>>,
}

impl<'env> ModuleGraphPendingSearchMap<'env> {
    pub fn get_or_default<Ret>(
        &self,
        graph_ref: ModuleGraphRef,
        f: impl FnOnce(&PendingSearchMap<'env>) -> Ret,
    ) -> Ret {
        loop {
            if let Some(value) = self.inner.read().unwrap().get(&graph_ref) {
                return f(value);
            }

            self.inner
                .write()
                .unwrap()
                .entry(graph_ref)
                .or_insert_with(|| Default::default());
        }
    }

    pub fn num_unresolved_symbol_references(&self) -> usize {
        self.inner
            .read()
            .unwrap()
            .values()
            .map(|pending_search_map| pending_search_map.num_unresolved_symbol_references())
            .sum()
    }
}

#[derive(Debug, Default)]
pub struct PendingSearchMap<'env> {
    inner: Mutex<PendingSearchMapInner<'env>>,
}

impl<'env> PendingSearchMap<'env> {
    // Returns the latest version approximation for a symbol.
    // This can be used to see if a pending symbol may be ready.
    // NOTE: May have false positives, but will never have a false negatives.
    pub fn get_pending_search_version(&self, name: &'env str) -> PendingSearchVersion {
        self.inner.lock().unwrap().get_pending_search_version(name)
    }

    // Returns `Ok` if the symbol is now being waiting on,
    // otherwise returns `Err` if the task should be woken immediately.
    pub fn suspend_on(
        &self,
        searched_version: PendingSearchVersion,
        name: &'env str,
        task_ref: TaskRef<'env>,
    ) -> Result<(), TaskRef<'env>> {
        self.inner
            .lock()
            .unwrap()
            .suspend_on(searched_version, name, task_ref)
    }

    // Returns which tasks should be woken up, since a new symbol has been added
    pub fn wake(&self, name: &'env str) -> Vec<TaskRef<'env>> {
        self.inner.lock().unwrap().wake(name)
    }

    pub fn num_unresolved_symbol_references(&self) -> usize {
        self.inner
            .lock()
            .unwrap()
            .num_unresolved_symbol_references()
    }
}

#[derive(Debug, Default)]
pub struct PendingSearchMapInner<'env> {
    map: HashMap<&'env str, PendingSearch<'env>>,
}

#[derive(Debug, Default)]
pub struct PendingSearch<'env> {
    tasks: Vec<TaskRef<'env>>,
    version: PendingSearchVersion,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PendingSearchVersion(pub usize);

impl PendingSearchVersion {
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

impl<'env> PendingSearchMapInner<'env> {
    // Returns the latest version approximation for a symbol.
    // This can be used to see if a pending symbol may be ready.
    // NOTE: May have false positives, but will never have a false negatives.
    pub fn get_pending_search_version(&mut self, name: &'env str) -> PendingSearchVersion {
        self.map.entry(name).or_default().version
    }

    // Returns `Ok` if the symbol is now being waiting on,
    // otherwise returns `Err` if the task should be woken immediately.
    pub fn suspend_on(
        &mut self,
        searched_version: PendingSearchVersion,
        name: &'env str,
        task_ref: TaskRef<'env>,
    ) -> Result<(), TaskRef<'env>> {
        let pending_search = self.map.get_mut(name).expect("cannot suspend on symbol without first getting the pending search version for that symbol");

        if searched_version < pending_search.version {
            Err(task_ref)
        } else {
            pending_search.tasks.push(task_ref);
            Ok(())
        }
    }

    // Returns which tasks should be woken up, since a new symbol has been added
    pub fn wake(&mut self, name: &'env str) -> Vec<TaskRef<'env>> {
        let pending_search = self.map.entry(name).or_default();
        pending_search.version.increment();
        std::mem::take(&mut pending_search.tasks)
    }

    pub fn num_unresolved_symbol_references(&self) -> usize {
        self.map
            .values()
            .map(|pending_search| pending_search.tasks.len())
            .sum()
    }
}
