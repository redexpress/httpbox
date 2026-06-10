use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl Method {
    pub fn all() -> [Method; 7] {
        [
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Delete,
            Method::Patch,
            Method::Head,
            Method::Options,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }

    pub fn as_reqwest(self) -> reqwest::Method {
        match self {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Delete => reqwest::Method::DELETE,
            Method::Patch => reqwest::Method::PATCH,
            Method::Head => reqwest::Method::HEAD,
            Method::Options => reqwest::Method::OPTIONS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    pub enabled: bool,
    pub key: String,
    pub value: String,
}

impl KeyValue {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            enabled: true,
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BodyKind {
    None,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthKind {
    None,
    Bearer,
    Basic,
}

impl Default for AuthKind {
    fn default() -> Self {
        AuthKind::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub kind: AuthKind,
    pub bearer_token: String,
    pub basic_user: String,
    pub basic_password: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            kind: AuthKind::None,
            bearer_token: String::new(),
            basic_user: String::new(),
            basic_password: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorTab {
    Body,
    Param,
    Auth,
}

impl Default for EditorTab {
    fn default() -> Self {
        EditorTab::Body
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseLayout {
    Bottom,
    Right,
}

impl Default for ResponseLayout {
    fn default() -> Self {
        ResponseLayout::Bottom
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub name: String,
    pub name_auto: bool,
    pub method: Method,
    pub url: String,
    pub query: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub body_kind: BodyKind,
    pub body_text: String,
    pub timeout_secs: u32,
    pub auth: AuthConfig,
    pub tab: EditorTab,
}

impl HttpRequest {
    pub fn new_named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            name_auto: true,
            method: Method::Get,
            url: String::new(),
            query: Vec::new(),
            headers: Vec::new(),
            body_kind: BodyKind::None,
            body_text: String::new(),
            timeout_secs: 30,
            auth: AuthConfig::default(),
            tab: EditorTab::Body,
        }
    }

    pub fn name_from_url(url: &str) -> String {
        let s = url.trim();
        if s.is_empty() {
            return String::new();
        }
        let mut rest = s;
        if let Some(idx) = rest.find("://") {
            rest = &rest[idx + 3..];
        }
        if let Some(idx) = rest.find('/') {
            rest = &rest[idx..];
        } else {
            rest = "";
        }
        rest.to_string()
    }
}

impl Default for HttpRequest {
    fn default() -> Self {
        Self::new_named("Request 1")
    }
}
