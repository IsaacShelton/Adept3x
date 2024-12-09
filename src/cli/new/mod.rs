mod invoke;
mod parse;

#[derive(Clone, Debug)]
pub struct NewCommand {
    pub project_name: String,
}
