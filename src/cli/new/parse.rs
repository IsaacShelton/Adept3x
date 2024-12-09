use super::NewCommand;

impl NewCommand {
    pub fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, ()> {
        // Skip over 'new' command keyword
        args.next().unwrap();

        let Some(project_name) = args.next() else {
            println!("adept new <PROJECT_NAME>");
            return Err(());
        };

        Ok(Self { project_name })
    }
}
