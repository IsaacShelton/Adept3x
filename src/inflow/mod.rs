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
pub use self::try_peek::TryPeek;
pub use self::tools::InflowTools;

pub trait Inflow<T>: InflowStream<Item = T> {
    fn peek_nth_mut<'a>(&'a mut self, n: usize) -> &'a mut T;

    fn peek_nth<'a>(&'a mut self, n: usize) -> &'a T {
        &*self.peek_nth_mut(n)
    }

    fn peek<'a>(&'a mut self) -> &'a T {
        self.peek_nth(0)
    }

    fn peek_mut<'a>(&'a mut self) -> &'a mut T {
        self.peek_nth_mut(0)
    }
}
