pub enum IteratorCoproduct2<L, R> {
    Left(L),
    Right(R),
}

impl<L, R, T> Iterator for IteratorCoproduct2<L, R>
where
    L: Iterator<Item = T>,
    R: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IteratorCoproduct2::Left(left) => left.next(),
            IteratorCoproduct2::Right(right) => right.next(),
        }
    }
}
