use source_files::SourceFiles;

pub trait Show {
    fn show(&self, w: &mut dyn std::fmt::Write, source_files: &SourceFiles) -> std::fmt::Result;

    fn eprintln(self: &Self, source_files: &SourceFiles) {
        let mut message = String::new();
        self.show(&mut message, source_files).unwrap();
        eprintln!("{}", message);
    }
}

pub fn into_show<T: Show + 'static>(show: T) -> Box<dyn Show> {
    Box::new(show)
}
