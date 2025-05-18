use super::Decl;
use smallvec::SmallVec;

/// A group of declarations under the same name
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeclSet(SmallVec<[Decl; 4]>);

impl<'env> DeclSet {
    pub fn push_unique(&mut self, decl: Decl) {
        self.0.push(decl);
    }
}
