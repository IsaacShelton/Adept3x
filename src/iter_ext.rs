pub trait MapSecond<T1, T2>: Iterator<Item = (T1, T2)> {
    fn map_second(self) -> impl Iterator<Item = T2>;
}

impl<T1, T2, I: Iterator<Item = (T1, T2)>> MapSecond<T1, T2> for I {
    fn map_second(self) -> impl Iterator<Item = T2> {
        self.map(|(_, second)| second)
    }
}
