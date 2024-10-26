#[derive(Copy, Clone, Debug)]
pub enum Initialized {
    Require,
    AllowUninitialized,
}
