use super::Execute;
use crate::{Artifact, Executor, Progress, repr::DeclScope};
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FindDeclSetByName<'env> {
    namespace: ByAddress<&'env DeclScope>,
    name: &'env str,
}

impl<'env> Execute<'env> for FindDeclSetByName<'env> {
    fn execute(self, _executor: &Executor<'env>) -> Progress<'env> {
        return Progress::complete(Artifact::Void);
    }
}
