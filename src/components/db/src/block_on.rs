#[derive(Copy, Clone, Debug)]
pub enum BlockOn<T> {
    Complete(T),
    Cyclic,
    TimedOut,
}
