use crate::{Type, poly_value::PolyValue};
use core::hash::Hash;
use indexmap::IndexMap;
use std::fmt::Display;

// TODO: We probably want this to store some kind of internal hash
// Also, it should itself implement hash
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolyRecipe {
    pub polymorphs: IndexMap<String, PolyValue>,
}

impl From<IndexMap<String, Type>> for PolyRecipe {
    fn from(mut value: IndexMap<String, Type>) -> Self {
        Self {
            polymorphs: IndexMap::from_iter(
                value
                    .drain(..)
                    .map(|(name, ty)| (name, PolyValue::Type(ty))),
            ),
        }
    }
}

impl Hash for PolyRecipe {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.polymorphs.len().hash(state);

        for (key, val) in self.polymorphs.iter() {
            key.hash(state);
            val.hash(state);
        }
    }
}

impl Display for PolyRecipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;

        for (i, (name, value)) in self.polymorphs.iter().enumerate() {
            write!(f, "${} :: ", name)?;

            // NOTE: We shouldn't need to do something like this for mangling, only concrete types,
            // impls, exprs, etc. should be included. We can probably be smarter than that too
            // though.
            match value {
                PolyValue::Type(ty) => {
                    write!(f, "{}", ty.to_string())?;
                }
                PolyValue::Expr(_) => {
                    todo!("mangle name for polymorphic function with expr polymorph")
                }
                PolyValue::Impl(impl_ref) => {
                    eprintln!(
                        "warning: name mangling for functions called with impl params is ad-hoc"
                    );
                    write!(f, "impl {:?}", impl_ref)?;
                }
                PolyValue::PolyImpl(name) => {
                    eprintln!(
                        "warning: name mangling for functions called with impl params is ad-hoc"
                    );
                    write!(f, "impl ${}", name)?;
                }
            }

            if i + 1 != self.polymorphs.len() {
                write!(f, ", ")?;
            }
        }

        write!(f, ")")?;
        Ok(())
    }
}
