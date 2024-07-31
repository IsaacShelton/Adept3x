pub enum FeedResult<T> {
    Has(T),
    Waiting,
    Eof(T),
}
