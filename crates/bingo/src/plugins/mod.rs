//! Plugin switches and the isolated author kit.
mod switch;
mod test;
mod validate;

use bingo_sdk::KernelError;
pub use switch::{Action, run};

pub async fn author(action: &Action) -> Option<Result<i32, KernelError>> {
    let result = match action {
        Action::Validate { dir } => validate::run(dir).await,
        Action::Test { dir, json } => test::run(dir, *json).await,
        _ => return None,
    };
    Some(match result {
        Ok(()) => Ok(0),
        Err(error) => {
            eprintln!("{}", error.message);
            Ok(1)
        }
    })
}

fn invalid(why: impl std::fmt::Display) -> KernelError {
    KernelError::new(bingo_sdk::ErrorCode::InvalidInput, why.to_string())
}
