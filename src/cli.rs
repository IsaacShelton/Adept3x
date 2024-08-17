use std::process::exit;

pub struct Command {
    pub kind: CommandKind,
}

impl Command {
    pub fn parse_env_args() -> Result<Self, ()> {
        let mut args = std::env::args().skip(1).peekable();

        match args.peek().map(|string| string.as_str()) {
            None | Some("-h" | "--help") => {
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
            if option == "-e" {
                options.excute_result = true;
            } else if option == "--emit-llvm-ir" {
                options.emit_llvm_ir = true;
            } else if option == "--emit-ir" {
                options.emit_ir = true;
            } else if option == "--interpret" {
                options.interpret = true;
                options.coerce_main_signature = false;
            } else if filename.is_some() {
                // TODO: Implement proper error handling and improve error message
                eprintln!("error: Multiple paths specified");
                return Err(());
            } else {
                filename = Some(option);
            }
        }

        let Some(filename) = filename else {
            // TODO: Implement proper error handling and improve error message
            eprintln!("error: No folder or filename specified");
            return Err(());
        };

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

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub emit_llvm_ir: bool,
    pub emit_ir: bool,
    pub interpret: bool,
    pub coerce_main_signature: bool,
    pub excute_result: bool,
    pub use_pic: Option<bool>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            emit_llvm_ir: false,
            emit_ir: false,
            interpret: false,
            coerce_main_signature: true,
            excute_result: false,
            use_pic: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewCommand {
    pub project_name: String,
}

fn show_help() {
    println!("usage: adept FILENAME");
}
