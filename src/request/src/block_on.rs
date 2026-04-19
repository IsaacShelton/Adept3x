use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum BlockOn<T> {
    Complete(T),
    Cyclic,
    Diverges,
    TimedOut,
}
