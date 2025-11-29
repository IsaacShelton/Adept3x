pub struct MacroState<S: Copy> {
    pub identifier: String,
    pub start_source: S,
}

impl<S: Copy> MacroState<S> {
    pub fn finalize(&mut self) -> (String, S) {
        let identifier = std::mem::take(&mut self.identifier);
        (identifier, self.start_source)
    }
}
