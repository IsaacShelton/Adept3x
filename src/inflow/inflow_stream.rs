use super::InflowEnd;

pub trait InflowStream {
    type Item: InflowEnd;

    fn next(&mut self) -> Self::Item;
}
