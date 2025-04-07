#![feature(maybe_uninit_array_assume_init)]

mod inflow_end;
mod inflow_stream;
mod into_inflow;
mod into_inflow_stream;
mod tools;

pub use self::{
    inflow_end::InflowEnd, inflow_stream::InflowStream, into_inflow::IntoInflow,
    into_inflow_stream::IntoInflowStream, tools::InflowTools,
};

pub trait Inflow<T>: InflowStream<Item = T> {
    fn un_next(&mut self, item: Self::Item);

    fn peek_nth_mut(&mut self, n: usize) -> &mut T;
    fn peek_n<const N: usize>(&mut self) -> [&T; N];

    fn peek_nth(&mut self, n: usize) -> &T {
        &*self.peek_nth_mut(n)
    }

    fn peek(&mut self) -> &T {
        self.peek_nth(0)
    }

    fn peek_mut(&mut self) -> &mut T {
        self.peek_nth_mut(0)
    }
}

impl<T: InflowEnd, I: Inflow<T>> Inflow<T> for &mut I {
    fn un_next(&mut self, item: Self::Item) {
        (**self).un_next(item)
    }

    fn peek_nth_mut(&mut self, n: usize) -> &mut T {
        (**self).peek_nth_mut(n)
    }

    fn peek_n<const N: usize>(&mut self) -> [&T; N] {
        (**self).peek_n::<N>()
    }
}
