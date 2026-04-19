pub trait IterTupleExt<T1, T2> {
    fn a(self) -> impl Iterator<Item = T1>;
    fn b(self) -> impl Iterator<Item = T2>;
}

pub trait IterTupleRefExt<'a, T1, T2>
where
    T1: 'a,
    T2: 'a,
{
    fn a(self) -> impl Iterator<Item = &'a T1>;
    fn b(self) -> impl Iterator<Item = &'a T2>;
}

pub trait IterTupleMutExt<'a, T1, T2>
where
    T1: 'a,
    T2: 'a,
{
    fn a(self) -> impl Iterator<Item = &'a mut T1>;
    fn b(self) -> impl Iterator<Item = &'a mut T2>;
}

impl<T1, T2, I> IterTupleExt<T1, T2> for I
where
    I: Iterator<Item = (T1, T2)>,
{
    fn a(self) -> impl Iterator<Item = T1> {
        self.map(|(a, _)| a)
    }

    fn b(self) -> impl Iterator<Item = T2> {
        self.map(|(_, b)| b)
    }
}

impl<'a, T1, T2, I> IterTupleRefExt<'a, T1, T2> for I
where
    I: Iterator<Item = &'a (T1, T2)>,
    T1: 'a,
    T2: 'a,
{
    fn a(self) -> impl Iterator<Item = &'a T1> {
        self.map(|(a, _)| a)
    }

    fn b(self) -> impl Iterator<Item = &'a T2> {
        self.map(|(_, b)| b)
    }
}

impl<'a, T1, T2, I> IterTupleMutExt<'a, T1, T2> for I
where
    I: Iterator<Item = &'a mut (T1, T2)>,
    T1: 'a,
    T2: 'a,
{
    fn a(self) -> impl Iterator<Item = &'a mut T1> {
        self.map(|(a, _)| a)
    }

    fn b(self) -> impl Iterator<Item = &'a mut T2> {
        self.map(|(_, b)| b)
    }
}
