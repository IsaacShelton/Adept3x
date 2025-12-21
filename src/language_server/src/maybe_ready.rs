use derive_more::From;

#[derive(Clone, Debug, From)]
pub enum MaybeReady<T> {
    Ready(T),
    Pending,
}
