use crate::{Cache, Pf, TaskStatus};

pub struct CacheEntry<'c, 'k, 'e, P: Pf> {
    pub(crate) key: &'k P::Req<'e>,
    pub(crate) cache: &'c mut Cache<'e, P>,
}

impl<'c, 'k, 'e, P: Pf> CacheEntry<'c, 'k, 'e, P> {
    pub fn or_insert_with(
        self,
        f: impl FnOnce() -> Option<TaskStatus<'e, P>>,
    ) -> &'c mut Option<TaskStatus<'e, P>> {
        // Why does Rust not have a better way to do this? Entry API requires pre-cloning...
        if self.cache.kv.inner.get(self.key).is_none() {
            self.cache.kv.inner.insert(self.key.clone(), (f)());
        }

        self.cache.kv.inner.get_mut(self.key).unwrap()
    }
}
