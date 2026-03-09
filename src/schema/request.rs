use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::auth::Auth;
use super::body::Body;
use super::method::HttpMethod;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteParam {
    pub segment: String,
    pub kind: RouteParamKind,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteParamKind {
    Integer,
    Uuid,
    Hex,
    Slug,
}

impl std::fmt::Display for RouteParamKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteParamKind::Integer => write!(f, ":id"),
            RouteParamKind::Uuid => write!(f, ":uuid"),
            RouteParamKind::Hex => write!(f, ":hex"),
            RouteParamKind::Slug => write!(f, ":slug"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurlRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Vec<Header>,
    pub route_params: Vec<RouteParam>,
    pub route_template: Option<String>,
    pub query_params: Vec<(String, serde_json::Value)>,
    pub query_schema: Option<serde_json::Value>,
    pub body: Option<Body>,
    pub body_schema: Option<serde_json::Value>,
    pub auth: Option<Auth>,
    pub cookies: Vec<Cookie>,
    pub follow_redirects: bool,
    pub max_redirects: Option<u32>,
    pub insecure: bool,
    pub compressed: bool,
    pub connect_timeout: Option<Duration>,
    pub max_time: Option<Duration>,
    pub proxy: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub output: Option<String>,
}

impl Default for CurlRequest {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: HttpMethod::GET,
            headers: Vec::new(),
            route_params: Vec::new(),
            route_template: None,
            query_params: Vec::new(),
            query_schema: None,
            body: None,
            body_schema: None,
            auth: None,
            cookies: Vec::new(),
            follow_redirects: false,
            max_redirects: None,
            insecure: false,
            compressed: false,
            connect_timeout: None,
            max_time: None,
            proxy: None,
            user_agent: None,
            referer: None,
            output: None,
        }
    }
}
