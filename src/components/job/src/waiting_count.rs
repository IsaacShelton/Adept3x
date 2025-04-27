#[derive(Debug, Default)]
pub struct WaitingCount(pub usize);

impl WaitingCount {
    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn decrement(&mut self) -> bool {
        self.0 -= 1;
        self.0 == 0
    }
}
