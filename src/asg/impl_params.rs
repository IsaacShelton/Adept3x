use super::GenericTraitRef;
use crate::index_map_ext::IndexMapExt;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct ImplParams {
    params: IndexMap<String, GenericTraitRef>,
}

impl ImplParams {
    pub fn new(params: IndexMap<String, GenericTraitRef>) -> Self {
        Self { params }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &GenericTraitRef)> {
        self.params.iter()
    }

    pub fn get(&self, key: &str) -> Option<&GenericTraitRef> {
        self.params.get(key)
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    pub fn has_items(&self) -> bool {
        self.params.has_items()
    }
}
