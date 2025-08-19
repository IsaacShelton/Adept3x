use source_files::{Source, SourceFiles};
use std::path::Path;

pub trait Show {
    fn show(
        &self,
        w: &mut dyn std::fmt::Write,
        source_files: &SourceFiles,
        project_root: Option<&Path>,
    ) -> std::fmt::Result;

    fn eprintln(self: &Self, source_files: &SourceFiles, project_root: Option<&Path>) {
        let mut message = String::new();
        self.show(&mut message, source_files, project_root).unwrap();
        eprintln!("{}", message);
    }
}

pub fn into_show<T: Show + 'static>(show: T) -> Box<dyn Show> {
    Box::new(show)
}

pub fn minimal_filename<'a>(
    source: Source,
    source_files: &'a SourceFiles,
    project_root: Option<&Path>,
) -> &'a str {
    let filename = source_files.get(source.key).filename();

    project_root
        .into_iter()
        .flat_map(|root| Path::new(filename).strip_prefix(root).ok())
        .next()
        .into_iter()
        .flat_map(|x| x.to_str())
        .next()
        .unwrap_or(filename)
}
