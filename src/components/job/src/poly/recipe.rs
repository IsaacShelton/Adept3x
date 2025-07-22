use super::PolyValue;
use indexmap::IndexMap;
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Clone, Debug, Default)]
pub struct PolyRecipe<'env> {
    polymorphs: IndexMap<&'env str, PolyValue<'env>>,
    hash_code: u64,
}

impl<'env> PolyRecipe<'env> {
    pub fn new(polymorphs: IndexMap<&'env str, PolyValue<'env>>) -> Self {
        Self {
            hash_code: hash_polymorphs(&polymorphs),
            polymorphs,
        }
    }
}

impl<'env> PartialEq for PolyRecipe<'env> {
    fn eq(&self, other: &Self) -> bool {
        if self.polymorphs.len() != other.polymorphs.len() {
            return false;
        }

        for ((a_name, a_value), (b_name, b_value)) in
            self.polymorphs.iter().zip(other.polymorphs.iter())
        {
            if a_name != b_name || a_value != b_value {
                return false;
            }
        }

        true
    }
}

impl<'env> Eq for PolyRecipe<'env> {}

impl<'env> Hash for PolyRecipe<'env> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash_code)
    }
}

impl<'env> FromIterator<(&'env str, PolyValue<'env>)> for PolyRecipe<'env> {
    fn from_iter<T: IntoIterator<Item = (&'env str, PolyValue<'env>)>>(iter: T) -> Self {
        let polymorphs = IndexMap::<&'env str, PolyValue<'env>>::from_iter(iter);
        let hash_code = hash_polymorphs(&polymorphs);

        Self {
            polymorphs,
            hash_code,
        }
    }
}

fn hash_polymorphs<'env>(polymorphs: &IndexMap<&'env str, PolyValue<'env>>) -> u64 {
    let random_state = polymorphs.hasher();
    let mut hasher = random_state.build_hasher();

    for (name, poly_value) in polymorphs.iter() {
        name.hash(&mut hasher);
        poly_value.hash(&mut hasher);
    }

    hasher.finish()
}
