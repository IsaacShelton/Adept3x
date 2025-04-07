mod from_iterator;

use super::{InflowEnd, InflowStream};
use from_iterator::InflowStreamFromIterator;

pub trait IntoInflowStream<T: InflowEnd> {
    fn into_inflow_stream(self, end: T) -> impl InflowStream<Item = T>;
}

impl<T: Clone + InflowEnd, I: Iterator<Item = T>> IntoInflowStream<T> for I {
    fn into_inflow_stream(self, end: T) -> impl InflowStream<Item = T> {
        InflowStreamFromIterator::new(self, end)
    }
}
