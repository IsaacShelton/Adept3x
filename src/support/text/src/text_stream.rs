use super::Character;

pub trait TextStream {
    fn next(&mut self) -> Character;
}
