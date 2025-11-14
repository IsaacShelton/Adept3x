use crate::Req;
use std_ext::SmallVec4;

pub trait WakeAfter {
    fn push(&mut self, req: Req);
    fn extend(&mut self, reqs: impl Iterator<Item = Req>);
}

impl WakeAfter for Vec<Req> {
    fn push(&mut self, req: Req) {
        self.push(req)
    }

    fn extend(&mut self, reqs: impl Iterator<Item = Req>) {
        Extend::extend(self, reqs)
    }
}

impl WakeAfter for SmallVec4<Req> {
    fn push(&mut self, req: Req) {
        self.push(req)
    }

    fn extend(&mut self, reqs: impl Iterator<Item = Req>) {
        Extend::extend(self, reqs)
    }
}
