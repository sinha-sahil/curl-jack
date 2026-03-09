use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Auth {
    Basic { username: String, password: String },
    Bearer(String),
}
