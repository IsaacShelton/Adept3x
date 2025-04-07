use crate::InflowEnd;

pub trait InflowStream {
    type Item: InflowEnd;

    fn next(&mut self) -> Self::Item;
}

impl<T: InflowEnd, I: InflowStream<Item = T>> InflowStream for &mut I {
    type Item = T;

    fn next(&mut self) -> Self::Item {
        (**self).next()
    }
}
