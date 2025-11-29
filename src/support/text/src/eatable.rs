pub trait Eatable {
    fn chars(self) -> impl Iterator<Item = char>;
}

impl Eatable for char {
    fn chars(self) -> impl Iterator<Item = char> {
        std::iter::once(self)
    }
}

impl Eatable for &str {
    fn chars(self) -> impl Iterator<Item = char> {
        self.chars()
    }
}
