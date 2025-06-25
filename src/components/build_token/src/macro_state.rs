use source_files::Source;

pub struct MacroState {
    pub identifier: String,
    pub start_source: Source,
}

impl MacroState {
    pub fn finalize(&mut self) -> (String, Source) {
        let identifier = std::mem::take(&mut self.identifier);
        (identifier, self.start_source)
    }
}
