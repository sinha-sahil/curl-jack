use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::schema::Header;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurlResponse {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
    pub elapsed: Duration,
}
