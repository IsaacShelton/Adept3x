use derive_more::Deref;

#[derive(Deref)]
pub struct RepeatingLast<I>
where
    I: Iterator,
    I::Item: Clone,
{
    #[deref]
    iterator: I,

    next_item: Option<I::Item>,
}

impl<I: Iterator<Item: Clone> + Clone> Clone for RepeatingLast<I> {
    fn clone(&self) -> Self {
        Self {
            iterator: self.iterator.clone(),
            next_item: self.next_item.clone(),
        }
    }
}

impl<I> RepeatingLast<I>
where
    I: Iterator,
    I::Item: Clone,
{
    pub fn new(mut iterator: I) -> Self {
        let next_item = iterator.next();

        Self {
            iterator,
            next_item,
        }
    }
}

impl<I> Iterator for RepeatingLast<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let new_item = self.iterator.next();

        if new_item.is_some() {
            std::mem::replace(&mut self.next_item, new_item)
        } else {
            self.next_item.clone()
        }
    }
}
