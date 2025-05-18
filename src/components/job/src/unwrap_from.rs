pub trait UnwrapFrom<T> {
    fn unwrap_from<'a>(from: &'a T) -> &'a Self;
}
