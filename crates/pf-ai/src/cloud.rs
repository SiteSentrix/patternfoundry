//! Foundry accounts: the requests the app sends the Foundry cloud service (the StitchFoundry
//! service, `apps/stitchfoundry-cloud` and `docs/cloud.md` in github.com/Stitchwright/StitchFoundry,
//! shared by every SiteSentrix app) to sign in, read the credits and buy more, and how an AI
//! request is sent through it, paid in credits, instead of to Anthropic with the user's own key
//! ([`through_cloud`]). As elsewhere in [`super`], no network code lives here.

use serde_json::{Value, json};

use super::{AiError, HttpRequest};

/// The secret the session token is kept as.
pub const TOKEN_NAME: &str = "foundry-session";
/// Session tokens start with this.
pub const TOKEN_PREFIX: &str = "sfs_";
/// The service's endpoint for AI requests.
pub const MESSAGES: &str = "/v1/messages";

/// `server` tidied (no trailing slash) when it is an address the app may send to: HTTPS, or
/// plain HTTP on this computer (for running the service locally).
pub fn clean_server(server: &str) -> Option<String> {
    let s = server.trim().trim_end_matches('/');
    let local = ["http://127.0.0.1", "http://localhost", "http://[::1]"]
        .iter()
        .any(|p| s.strip_prefix(p).is_some_and(|rest| rest.is_empty() || rest.starts_with(':') || rest.starts_with('/')));
    let ok = (s.starts_with("https://") && s.len() > "https://".len() || local) && !s.chars().any(|c| c.is_whitespace() || c.is_control());
    ok.then(|| s.to_string())
}

/// A POST of `body` to `path` on `server`, signed in with `token` when given.
pub fn request(server: &str, path: &str, token: Option<&str>, body: &Value) -> Result<HttpRequest, AiError> {
    let server = clean_server(server).ok_or(AiError::NoServer)?;
    let mut headers = vec![("content-type", "application/json".to_string())];
    if let Some(t) = token {
        headers.push(("authorization", format!("Bearer {}", t.trim())));
    }
    Ok(HttpRequest { url: format!("{server}{path}"), headers, body: body.to_string() })
}

pub fn sign_in(server: &str, email: &str, password: &str) -> Result<HttpRequest, AiError> {
    request(server, "/v1/signin", None, &json!({ "email": email.trim(), "password": password }))
}

pub fn sign_up(server: &str, email: &str, password: &str) -> Result<HttpRequest, AiError> {
    request(server, "/v1/signup", None, &json!({ "email": email.trim(), "password": password }))
}

pub fn sign_out(server: &str, token: &str) -> Result<HttpRequest, AiError> {
    request(server, "/v1/signout", Some(token), &json!({}))
}

/// The account: `{email, credits, packs: [{id, credits, cents}], payments, history}`.
pub fn account(server: &str, token: &str) -> Result<HttpRequest, AiError> {
    request(server, "/v1/account", Some(token), &json!({}))
}

/// A checkout page for pack `pack`: `{url}` to open in the browser.
pub fn checkout(server: &str, token: &str, pack: &str) -> Result<HttpRequest, AiError> {
    request(server, "/v1/checkout", Some(token), &json!({ "pack": pack }))
}

/// The error an answer from the service with HTTP status `status` and body `body` means.
pub fn status_error(status: u16, body: &str) -> AiError {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    match status {
        401 | 403 => AiError::SignIn,
        402 => AiError::NoCredits,
        409 => AiError::Taken,
        429 | 529 => AiError::Busy,
        503 => AiError::CloudClosed,
        s if s >= 500 => AiError::Service(s),
        s => AiError::Request(s, message),
    }
}

/// An answer from the service: its JSON, or what went wrong.
pub fn parse(status: u16, body: &str) -> Result<Value, AiError> {
    if !(200..300).contains(&status) {
        return Err(status_error(status, body));
    }
    serde_json::from_str(body).map_err(|_| AiError::NoAnswer)
}

/// `http`, an AI request built for Anthropic with session token `token` in place of an API key
/// (`x-api-key`), sent instead to `path` ([`MESSAGES`]) on the service at `server`, which pays
/// for it in credits, with `extra` headers.
pub fn through_cloud(mut http: HttpRequest, server: &str, path: &str, extra: Vec<(&'static str, String)>) -> Result<HttpRequest, AiError> {
    let server = clean_server(server).ok_or(AiError::NoServer)?;
    let token =
        http.headers.iter().find(|h| h.0 == "x-api-key").map(|h| h.1.clone()).filter(|t| t.starts_with(TOKEN_PREFIX)).ok_or(AiError::SignIn)?;
    http.headers.retain(|h| h.0 != "x-api-key");
    http.headers.push(("authorization", format!("Bearer {token}")));
    http.headers.extend(extra);
    http.url = format!("{server}{path}");
    Ok(http)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_or_this_computer() {
        assert_eq!(clean_server(" https://cloud.example.com/ ").as_deref(), Some("https://cloud.example.com"));
        assert_eq!(clean_server("http://127.0.0.1:8080").as_deref(), Some("http://127.0.0.1:8080"));
        assert_eq!(clean_server("http://localhost").as_deref(), Some("http://localhost"));
        for bad in ["", "https://", "http://cloud.example.com", "ftp://x", "http://localhost.evil.com", "https://a b.com"] {
            assert_eq!(clean_server(bad), None, "{bad}");
        }
    }

    #[test]
    fn requests_and_answers() {
        let r = sign_in("https://c.example", " denise@example.com ", "fire truck 1").unwrap();
        assert_eq!(r.url, "https://c.example/v1/signin");
        let b: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(b, json!({"email": "denise@example.com", "password": "fire truck 1"}));
        assert!(r.headers.iter().all(|h| h.0 != "authorization"));
        let r = checkout("https://c.example", "sfs_abc", "small").unwrap();
        assert!(r.headers.contains(&("authorization", "Bearer sfs_abc".to_string())));
        assert_eq!(account("", "sfs_abc"), Err(AiError::NoServer));
        assert_eq!(parse(200, r#"{"credits": 5}"#).unwrap()["credits"], 5);
        assert_eq!(parse(402, r#"{"type":"error","error":{"type":"x","message":"not enough credits"}}"#), Err(AiError::NoCredits));
        assert_eq!(parse(401, "{}"), Err(AiError::SignIn));
        assert_eq!(parse(409, "{}"), Err(AiError::Taken));
        assert_eq!(parse(503, "{}"), Err(AiError::CloudClosed));
        assert_eq!(
            parse(400, r#"{"error":{"message":"a password needs at least 8 characters"}}"#),
            Err(AiError::Request(400, "a password needs at least 8 characters".into()))
        );
    }

    #[test]
    fn an_ai_request_goes_through_the_service_with_the_session() {
        let q = super::super::ask::question("Which seam allowance?", None).unwrap();
        let http = super::super::ask::request("sfs_token", &[q]).unwrap();
        let body = http.body.clone();
        let c = through_cloud(http, "https://c.example/", MESSAGES, vec![]).unwrap();
        assert_eq!(c.url, "https://c.example/v1/messages");
        assert!(c.headers.iter().all(|h| h.0 != "x-api-key"));
        assert!(c.headers.contains(&("authorization", "Bearer sfs_token".to_string())));
        assert!(c.headers.iter().any(|h| h.0 == "anthropic-version"));
        assert_eq!(c.body, body, "the same request");
        // an Anthropic key never goes to the service
        let own = super::super::ask::request("sk-ant-mine", &[super::super::ask::question("x", None).unwrap()]).unwrap();
        assert_eq!(through_cloud(own, "https://c.example", MESSAGES, vec![]), Err(AiError::SignIn));
    }
}
