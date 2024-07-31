mod inflow_end;
mod inflow_stream;
mod into_inflow;
mod into_inflow_stream;
mod tools;
mod try_peek;

pub use self::inflow_end::InflowEnd;
pub use self::inflow_stream::InflowStream;
pub use self::into_inflow::IntoInflow;
pub use self::into_inflow_stream::IntoInflowStream;
pub use self::tools::InflowTools;
pub use self::try_peek::TryPeek;

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
