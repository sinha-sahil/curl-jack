use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Body {
    Raw(String),
    Json(serde_json::Value),
    Binary(Vec<u8>),
    FormUrlencoded(Vec<(String, serde_json::Value)>),
    Multipart(Vec<FormField>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub value: FormValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormValue {
    Text(String),
    File { path: String, mime: Option<String> },
}
