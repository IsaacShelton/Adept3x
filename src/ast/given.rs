use super::Type;

#[derive(Clone, Debug)]
pub struct Given {
    pub name: Option<String>,
    pub ty: Type,
}
