use super::{InflowEnd, InflowStream};

pub trait InflowTools<T: InflowEnd>: InflowStream<Item = T> {
    fn collect_vec(&mut self, keep_end: bool) -> Vec<T> {
        let mut collected = vec![];

        loop {
            let item = self.next();

            if item.is_inflow_end() {
                if keep_end {
                    collected.push(item);
                }
                return collected;
            }

            collected.push(item);
        }
    }
}

impl<T: InflowEnd, S: InflowStream<Item = T>> InflowTools<T> for S {}
