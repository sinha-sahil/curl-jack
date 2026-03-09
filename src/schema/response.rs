use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::request::Header;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedResponse {
    pub status: u16,
    pub status_text: String,
    pub status_class: StatusClass,
    pub headers: Vec<Header>,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub body: ResponseBody,
    pub body_schema: Option<serde_json::Value>,
    pub cookies: Vec<ResponseCookie>,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusClass {
    Informational,
    Success,
    Redirection,
    ClientError,
    ServerError,
    Unknown,
}

impl StatusClass {
    pub fn from_status(code: u16) -> Self {
        match code {
            100..=199 => StatusClass::Informational,
            200..=299 => StatusClass::Success,
            300..=399 => StatusClass::Redirection,
            400..=499 => StatusClass::ClientError,
            500..=599 => StatusClass::ServerError,
            _ => StatusClass::Unknown,
        }
    }
}

impl std::fmt::Display for StatusClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusClass::Informational => write!(f, "Informational"),
            StatusClass::Success => write!(f, "Success"),
            StatusClass::Redirection => write!(f, "Redirection"),
            StatusClass::ClientError => write!(f, "Client Error"),
            StatusClass::ServerError => write!(f, "Server Error"),
            StatusClass::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseBody {
    Json(serde_json::Value),
    Text(String),
    Html(String),
    Xml(String),
    Binary { size: usize },
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseCookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<String>,
    pub max_age: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<String>,
}
