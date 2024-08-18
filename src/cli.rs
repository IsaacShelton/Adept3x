use crate::target::{Target, TargetOs};
use std::{path::PathBuf, process::exit, str::FromStr};

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

    fn parse_build_project(mut args: impl Iterator<Item = String>) -> Result<Self, ()> {
        let mut filename = None;
        let mut options = BuildOptions::default();

        while let Some(option) = args.next() {
            match option.as_str() {
                "-e" => options.excute_result = true,
                "--emit-ir" => options.emit_ir = true,
                "--emit-llvm-ir" => options.emit_llvm_ir = true,
                "--interpret" => {
                    options.interpret = true;
                    options.coerce_main_signature = false;
                }
                "--windows" => {
                    options.target = Target::generic_os(TargetOs::Windows);
                }
                "--mac" | "--macos" => {
                    options.target = Target::generic_os(TargetOs::Mac);
                }
                "--linux" => {
                    options.target = Target::generic_os(TargetOs::Linux);
                }
                "--infrastructure" => {
                    let Some(infrastructure) = args.next() else {
                        eprintln!("error: Expected infrastructure path after '--infrastructure'");
                        return Err(());
                    };

                    options.infrastructure = Some(
                        PathBuf::from_str(&infrastructure)
                            .expect("invalid non-utf-8 infrastructure path"),
                    );
                }
                _ => {
                    if filename.replace(option).is_some() {
                        eprintln!("error: Multiple paths specified");
                        return Err(());
                    }
                }
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
    pub target: Target,
    pub infrastructure: Option<PathBuf>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        let current_exe = std::env::current_exe()
            .expect("failed to get adept executable location")
            .parent()
            .expect("parent folder")
            .to_path_buf();

        let infrastructure = current_exe.join("infrastructure");

        Self {
            emit_llvm_ir: false,
            emit_ir: false,
            interpret: false,
            coerce_main_signature: true,
            excute_result: false,
            use_pic: None,
            target: Target::HOST,
            infrastructure: Some(infrastructure),
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
