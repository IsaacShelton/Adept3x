mod peeker;

use crate::{Inflow, InflowEnd, InflowStream};
use peeker::InflowPeeker;

pub trait IntoInflow<T>: InflowStream<Item = T> {
    fn into_inflow(self) -> impl Inflow<T>;
}

impl<T: InflowEnd, S: InflowStream<Item = T>> IntoInflow<T> for S {
    fn into_inflow(self) -> impl Inflow<T> {
        InflowPeeker::new(self)
    }
}
