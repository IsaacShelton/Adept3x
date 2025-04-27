#[derive(Debug)]
pub enum Artifact {
    Void,
    String(String),
}

impl Artifact {
    pub fn unwrap_string(&self) -> &str {
        if let Self::String(string) = self {
            return string;
        }

        panic!("Expected execution artifact to be string");
    }
}
