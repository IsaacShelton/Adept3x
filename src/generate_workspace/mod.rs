/*
    =======================  generate_workspace/mod.rs  =======================
    Module for generating new workspaces
    ---------------------------------------------------------------------------
*/

use crate::cli::NewCommand;
use indoc::indoc;
use std::{borrow::Borrow, fs, path::Path};

pub fn new_project(new_command: NewCommand) -> Result<(), ()> {
    if std::fs::create_dir(&new_command.project_name).is_err() {
        eprintln!(
            "error: Failed to create project directory '{}'",
            &new_command.project_name
        );
        return Err(());
    }

    let folder = Path::new(&new_command.project_name);

    put_file(
        folder.join("_.adept"),
        indoc! {r#"

            pragma => {
                adept("3.0")
            }
        "#},
    )?;

    put_file(
        folder.join("main.adept"),
        indoc! {r#"

            func main {
                println("Hello World!")
            }
        "#},
    )?;

    println!("Project created!");
    Ok(())
}

fn put_file(path: impl Borrow<Path>, content: &str) -> Result<(), ()> {
    let path = path.borrow();

    if fs::write(path, content).is_err() {
        let error_filename = path
            .file_name()
            .and_then(|filename| filename.to_str())
            .unwrap_or("<invalid unicode filename>");

        eprintln!("error: Failed to create '{}' file", error_filename);
        return Err(());
    }

    Ok(())
}
