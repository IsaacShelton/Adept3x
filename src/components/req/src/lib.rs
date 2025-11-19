pub use requests::*;

#[define_requests::group]
mod requests {
    #[define_requests::returns(String)]
    pub struct GetRootSourceFile;
    pub struct GetRootSourceFileState;

    #[define_requests::returns(())]
    pub struct Approach;
    pub struct ApproachState;

    #[define_requests::returns(String)]
    pub struct Search();
    pub struct SearchState;

    #[define_requests::returns(Vec<String>)]
    pub struct Whatever;
    pub struct WhateverState;
}
