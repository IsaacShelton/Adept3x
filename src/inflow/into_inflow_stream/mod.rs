mod from_iterator;

use self::from_iterator::InflowStreamFromIterator;
use super::{InflowEnd, InflowStream};

pub trait IntoInflowStream<T: InflowEnd> {
    fn into_inflow_stream(self, end: T) -> impl InflowStream<Item = T>;
}

impl<T: Clone + InflowEnd, I: Iterator<Item = T>> IntoInflowStream<T> for I {
    fn into_inflow_stream(self, end: T) -> impl InflowStream<Item = T> {
        InflowStreamFromIterator::new(self, end)
    }
}
