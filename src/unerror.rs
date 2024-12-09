use crate::{show::Show, source_files::SourceFiles};

pub fn unerror<T, E: Show>(result: Result<T, E>, source_files: &SourceFiles) -> Result<T, ()> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            let mut message = String::new();

            err.show(&mut message, source_files)
                .expect("show error message");

            eprintln!("{message}");
            Err(())
        }
    }
}
