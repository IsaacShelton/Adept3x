mod peeker;

use self::peeker::InflowPeeker;
use super::{Inflow, InflowEnd, InflowStream};

pub trait IntoInflow<T>: InflowStream<Item = T> {
    fn into_inflow(self) -> impl Inflow<T>;
}

impl<T: InflowEnd, S: InflowStream<Item = T>> IntoInflow<T> for S {
    fn into_inflow(self) -> impl Inflow<T> {
        InflowPeeker::new(self)
    }
}
