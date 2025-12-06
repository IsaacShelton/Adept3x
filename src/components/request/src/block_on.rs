#[derive(Copy, Clone, Debug)]
pub enum BlockOn<T, Q> {
    Complete(T),
    Cyclic,
    Diverges,
    TimedOut(Q),
}
