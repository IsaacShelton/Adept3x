use super::{Asg, TraitRef, Type};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct GenericTraitRef {
    pub trait_ref: TraitRef,
    pub args: Vec<Type>,
}

impl GenericTraitRef {
    pub fn display<'a>(&'a self, asg: &'a Asg) -> DisplayGenericTraitRef<'a> {
        DisplayGenericTraitRef {
            generic_trait: self,
            asg,
        }
    }
}

pub struct DisplayGenericTraitRef<'a> {
    generic_trait: &'a GenericTraitRef,
    asg: &'a Asg<'a>,
}

impl<'a> Display for DisplayGenericTraitRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trait_decl = self
            .asg
            .traits
            .get(self.generic_trait.trait_ref)
            .expect("referenced trait to exist");

        write!(f, "{}", trait_decl.human_name.0)?;

        if self.generic_trait.args.is_empty() {
            return Ok(());
        }

        write!(f, "<")?;

        for (i, ty) in self.generic_trait.args.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }

            write!(f, "{}", ty)?;
        }

        write!(f, ">")?;
        Ok(())
    }
}
