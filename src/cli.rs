use std::process::exit;

pub struct Command {
    pub kind: CommandKind,
}

impl Command {
    pub fn parse_env_args() -> Result<Self, ()> {
        let mut args = std::env::args().skip(1).peekable();

        match args.peek().map(|string| string.as_str()) {
            None | Some("-h") | Some ("--help") => {
                show_help();
                exit(0);
            }
            Some("new") => Self::parse_new_project(args),
            _ => Self::parse_build_project(args),
        }
    }


    fn parse_build_project(mut args: impl Iterator<Item = String>) -> Result<Self, ()> {
        let filename = args.next().expect("filename to be specified");

        Ok(Self {
            kind: CommandKind::Build(BuildCommand {
                filename,
            })
        })
    }

    fn parse_new_project(mut args: impl Iterator<Item = String>) -> Result<Self, ()> {
        // Skip over 'new' command keyword
        args.next().unwrap();

        let project_name = match args.next() {
            Some(project_name) => project_name,
            None => {
                println!("adept new <PROJECT_NAME>");
                return Err(())
            },
        };

        Ok(Self {
            kind: CommandKind::New(NewCommand {
                project_name
            }),
        })
    }
}

pub enum CommandKind {
    Build(BuildCommand),
    New(NewCommand),
}

pub struct BuildCommand {
    pub filename: String,
}

pub struct NewCommand {
    pub project_name: String,
}

fn show_help() {
    println!("usage: adept FILENAME");
}
