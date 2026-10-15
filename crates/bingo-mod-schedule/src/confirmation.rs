use bingo_services::schedule::Row;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Confirmation {
    Forget(Row),
    Confirm(Row),
}
impl Confirmation {
    pub fn row(&self) -> &Row {
        match self {
            Self::Forget(row) | Self::Confirm(row) => row,
        }
    }
    pub fn verb(&self) -> &str {
        match self {
            Self::Forget(_) => "forget",
            Self::Confirm(_) => "confirm",
        }
    }
}
