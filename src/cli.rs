use std::process::exit;

pub struct Command {
    pub kind: CommandKind,
}

impl Command {
    pub fn parse_env_args() -> Result<Self, ()> {
        let mut args = std::env::args().skip(1).peekable();

        match args.peek().map(|string| string.as_str()) {
            None | Some("-h") | Some("--help") => {
                show_help();
                exit(0);
            }
            Some("new") => Self::parse_new_project(args),
            _ => Self::parse_build_project(args),
        }
    }

    fn parse_build_project(args: impl Iterator<Item = String>) -> Result<Self, ()> {
        let mut filename = None;
        let mut options = BuildOptions::default();

        for option in args {
            if option == "--emit-llvm-ir" {
                options.emit_llvm_ir = true;
            } else if option == "--emit-ir" {
                options.emit_ir = true;
            } else if option == "--interpret" {
                options.interpret = true;
            } else if filename.is_some() {
                // TODO: Implement proper error handling and improve error message
                eprintln!("error: Multiple paths specified");
                return Err(());
            } else {
                filename = Some(option);
            }
        }

        // TODO: Implement proper error handling and improve error message
        let filename = filename.expect("filename to be specified");

        Ok(Self {
            kind: CommandKind::Build(BuildCommand { filename, options }),
        })
    }

    fn parse_new_project(mut args: impl Iterator<Item = String>) -> Result<Self, ()> {
        // Skip over 'new' command keyword
        args.next().unwrap();

        let project_name = match args.next() {
            Some(project_name) => project_name,
            None => {
                println!("adept new <PROJECT_NAME>");
                return Err(());
            }
        };

        Ok(Self {
            kind: CommandKind::New(NewCommand { project_name }),
        })
    }
}

#[derive(Clone, Debug)]
pub enum CommandKind {
    Build(BuildCommand),
    New(NewCommand),
}

#[derive(Clone, Debug)]
pub struct BuildCommand {
    pub filename: String,
    pub options: BuildOptions,
}

#[derive(Clone, Debug, Default)]
pub struct BuildOptions {
    pub emit_llvm_ir: bool,
    pub emit_ir: bool,
    pub interpret: bool,
}

#[derive(Clone, Debug)]
pub struct NewCommand {
    pub project_name: String,
}

fn show_help() {
    println!("usage: adept FILENAME");
}
