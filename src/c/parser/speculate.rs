
macro_rules! speculate {
    ($input:expr, $expression:expr) => {{
        $input.speculate();

        match $expression {
            Ok(ok) => {
                $input.success();
                Ok(ok)
            }
            Err(err) => {
                $input.backtrack();
                Err(err)
            }
        }
    }};
}

pub(crate) use speculate;
