use super::NewCommand;
use crate::cli::CliInvoke;
use indoc::indoc;
use std::path::Path;

impl CliInvoke for NewCommand {
    fn invoke(self) -> Result<(), ()> {
        if std::fs::create_dir(&self.project_name).is_err() {
            eprintln!(
                "error: Failed to create project directory '{}'",
                &self.project_name
            );
            return Err(());
        }

        let folder = Path::new(&self.project_name);

        put_file(
            &folder.join("_.adept"),
            indoc! {r#"

            pragma => {
                adept("3.0")
            }
        "#},
        )?;

        put_file(
            &folder.join("main.adept"),
            indoc! {r#"

            func main {
                println("Hello World!")
            }
        "#},
        )?;

        println!("Project created!");
        Ok(())
    }
}

fn put_file(path: &Path, content: &str) -> Result<(), ()> {
    std::fs::write(path, content).map_err(|_| {
        let error_filename = path
            .file_name()
            .and_then(|filename| filename.to_str())
            .unwrap_or("<invalid unicode filename>");

        eprintln!("error: Failed to create '{}' file", error_filename)
    })
}
