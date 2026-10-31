//! MakerFoundry (PF-11): publishing the document to MakerFoundry, the SiteSentrix marketplace, as
//! a private draft the person finishes on MakerFoundry's own page. The contract is MakerFoundry's
//! `docs/api.md` ("How publishing from an app works", github.com/SiteSentrix/makerfoundry):
//!
//! - `POST https://makerfoundry.app/api/v1/designs`, `Authorization: Bearer <the person's Foundry
//!   session, sfs_…>` (never any other key), JSON `{file_name, file (base64), app}`;
//! - 201 `{id, url, kind, format}`: the draft, opened in the browser only when it is on the site
//!   the request went to; errors as `{"error": {"message"}}`, whose sentence is shown as it is.
//!
//! The document goes as SVG for now, a vector design of MakerFoundry's laser kind (its title from
//! the file name); a sewing-pattern kind (true-scale and tiled PDF, DXF-AAMA) is queued on
//! MakerFoundry's side, and [`publish`] takes any file name and contents so those can follow. As
//! elsewhere in [`super`], no network code lives here.

use serde_json::{Value, json};

use super::HttpRequest;
use super::cloud::TOKEN_PREFIX;

/// MakerFoundry's site: where designs are sent, and the only site whose pages are opened.
pub const SITE: &str = "https://makerfoundry.app";
/// The site's host.
pub const HOST: &str = "makerfoundry.app";
/// The endpoint designs are published to.
pub const DESIGNS: &str = "/api/v1/designs";
/// The most a design file may weigh (MakerFoundry's limit for vector files).
pub const MAX_FILE: usize = 8 * 1024 * 1024;
/// The longest address or file name taken from an answer.
const MAX_ADDRESS: usize = 2048;
/// The longest sentence of MakerFoundry's shown.
const MAX_SENTENCE: usize = 300;
/// The longest title a file name keeps (MakerFoundry's slugs keep 60).
const MAX_TITLE: usize = 60;

/// Why a design wasn't published. The messages are for the user; the UI shows them translated,
/// and MakerFoundry's own sentences as they are.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum MfError {
    #[error("Sign in to your Foundry Account first: Help › Foundry Account…")]
    SignIn,
    #[error("The document is empty: there is nothing to publish.")]
    Empty,
    #[error("The design is too large for MakerFoundry: at most 8 MB.")]
    TooLarge,
    #[error("MakerFoundry's answer couldn't be read.")]
    NoAnswer,
    #[error("MakerFoundry answered with a page on another site, which wasn't opened.")]
    OffSite,
    /// MakerFoundry's own sentence (with the HTTP status it came with).
    #[error("MakerFoundry: {1}")]
    Said(u16, String),
    #[error("MakerFoundry couldn't be reached: {0}")]
    Network(String),
}

/// A published draft: its id and the page where the person finishes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Published {
    pub id: u64,
    pub url: String,
}

/// The file name a design titled `title` goes as, ending in `.svg` (`ext`): MakerFoundry makes the
/// draft's title from it ("Simple_tote-bag.svg" → "Simple tote bag"). Letters and digits are
/// kept (any script), runs of anything else become one `_`, at most [`MAX_TITLE`] characters;
/// a title that leaves nothing is "PatternFoundry_design".
pub fn file_name(title: &str, ext: &str) -> String {
    let stem = title.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = match stem.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric()) && e.len() <= 16 => s,
        _ => stem,
    };
    let mut out = String::new();
    for c in stem.chars() {
        if c.is_alphanumeric() || c == '-' {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let out: String = out.trim_matches(['_', '-']).chars().take(MAX_TITLE).collect();
    let out = out.trim_end_matches(['_', '-']);
    let stem = if out.chars().any(char::is_alphanumeric) { out } else { "PatternFoundry_design" };
    format!("{stem}.{}", ext.trim_start_matches('.'))
}

/// The request publishing `file_base64` (the file's bytes, `len` of them, base64-encoded) as
/// `file_name`, signed in with the Foundry session `token`, from app `app` ("PatternFoundry
/// 0.13.1"). Only a session token is ever sent: anything else is refused before it leaves.
pub fn publish(token: &str, file_name: &str, file_base64: &str, len: usize, app: &str) -> Result<HttpRequest, MfError> {
    let token = token.trim();
    if !token.starts_with(TOKEN_PREFIX) || token.len() <= TOKEN_PREFIX.len() || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(MfError::SignIn);
    }
    if len == 0 || file_base64.is_empty() {
        return Err(MfError::Empty);
    }
    if len > MAX_FILE {
        return Err(MfError::TooLarge);
    }
    let body = json!({ "file_name": file_name, "file": file_base64, "app": app });
    Ok(HttpRequest {
        url: format!("{SITE}{DESIGNS}"),
        headers: vec![("content-type", "application/json".to_string()), ("authorization", format!("Bearer {token}"))],
        body: body.to_string(),
    })
}

/// MakerFoundry's answer to [`publish`] with HTTP status `status` and body `body`: the draft, or
/// why not (its own sentence for a refusal).
pub fn published(status: u16, body: &str) -> Result<Published, MfError> {
    if !(200..300).contains(&status) {
        return Err(refusal(status, body));
    }
    let v: Value = serde_json::from_str(body).map_err(|_| MfError::NoAnswer)?;
    let id = v["id"].as_u64().ok_or(MfError::NoAnswer)?;
    let url = v["url"].as_str().ok_or(MfError::NoAnswer)?;
    if !on_site(url) {
        return Err(MfError::OffSite);
    }
    Ok(Published { id, url: url.to_string() })
}

/// The error a refusal with HTTP status `status` and body `body` is: MakerFoundry's own sentence
/// (`{"error": {"message"}}`), tidied (one line, at most [`MAX_SENTENCE`] characters), else what
/// its status means.
pub fn refusal(status: u16, body: &str) -> MfError {
    let said = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(sentence))
        .filter(|s| !s.is_empty());
    match said {
        Some(s) => MfError::Said(status, s),
        None if status == 401 => MfError::SignIn,
        None if status == 413 => MfError::TooLarge,
        None => MfError::Said(status, format!("HTTP {status}")),
    }
}

/// `s` as one line of at most [`MAX_SENTENCE`] characters, without control characters.
fn sentence(s: &str) -> String {
    let one: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let one = one.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > MAX_SENTENCE { one.chars().take(MAX_SENTENCE - 1).chain(['…']).collect() } else { one }
}

/// Whether `url` is a page on MakerFoundry's site, the one the request went to: `https://`, the
/// host exactly [`HOST`] (no user name, no port, no other host ending in it), then a path; printable
/// ASCII without spaces or backslashes, at most [`MAX_ADDRESS`] characters.
pub fn on_site(url: &str) -> bool {
    if url.len() > MAX_ADDRESS || !url.bytes().all(|b| b.is_ascii_graphic() && b != b'\\') {
        return false;
    }
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    &rest[..host_end] == HOST && rest[host_end..].starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_design_is_sent_with_the_session_and_nothing_else() {
        let r = publish(" sfs_abc ", "Skirt.svg", "PHN2Zy8+", 6, "PatternFoundry 0.13.1").unwrap();
        assert_eq!(r.url, "https://makerfoundry.app/api/v1/designs");
        assert!(r.headers.contains(&("authorization", "Bearer sfs_abc".to_string())));
        assert_eq!(r.headers.iter().filter(|h| h.0 == "authorization").count(), 1);
        let body: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(body, json!({"file_name": "Skirt.svg", "file": "PHN2Zy8+", "app": "PatternFoundry 0.13.1"}));
        // An Anthropic key, an empty token, or one with a line break in it never leaves.
        for bad in ["sk-ant-api03-abc", "", "sfs_", "sfs_a\r\nx-api-key: 1", "Bearer sfs_x"] {
            assert_eq!(publish(bad, "a.svg", "PA==", 1, "PatternFoundry"), Err(MfError::SignIn), "{bad:?}");
        }
        assert_eq!(publish("sfs_a", "a.svg", "", 0, "PatternFoundry"), Err(MfError::Empty));
        assert_eq!(publish("sfs_a", "a.svg", "PA==", MAX_FILE + 1, "PatternFoundry"), Err(MfError::TooLarge));
        assert!(publish("sfs_a", "a.svg", "PA==", MAX_FILE, "PatternFoundry").is_ok());
    }

    #[test]
    fn file_names_make_meaningful_titles() {
        assert_eq!(file_name("Simple tote bag.patternfoundry", "svg"), "Simple_tote_bag.svg");
        assert_eq!(file_name("/home/me/A-line skirt (v2).svg", ".svg"), "A-line_skirt_v2.svg");
        assert_eq!(file_name("C:\\pats\\Röck.pdf", "svg"), "Röck.svg");
        assert_eq!(file_name("Untitled-1", "svg"), "Untitled-1.svg");
        for nothing in ["", "...", "/", "___", "..svg", "\u{202e}\u{0}"] {
            assert_eq!(file_name(nothing, "svg"), "PatternFoundry_design.svg", "{nothing:?}");
        }
        let long = file_name(&"x".repeat(500), "svg");
        assert_eq!(long, format!("{}.svg", "x".repeat(60)));
        // Nothing that could leave the name: no separators, dots or controls in the stem.
        let n = file_name("../../etc/passwd\n.svg", "svg");
        assert_eq!(n, "passwd.svg");
    }

    #[test]
    fn the_draft_opens_only_on_makerfoundrys_own_site() {
        let ok = published(201, r#"{"id": 7, "url": "https://makerfoundry.app/designs/7/edit", "kind": "laser", "format": "svg"}"#);
        assert_eq!(ok, Ok(Published { id: 7, url: "https://makerfoundry.app/designs/7/edit".into() }));
        for url in [
            "https://makerfoundry.app.evil.com/designs/7",
            "https://evil.com/makerfoundry.app/",
            "https://makerfoundry.app@evil.com/designs/7",
            "https://user@makerfoundry.app/designs/7",
            "https://makerfoundry.app:8443/designs/7",
            "https://makerfoundry.app./designs/7",
            "https://www.makerfoundry.app/designs/7",
            "https://MAKERFOUNDRY.APP/designs/7",
            "http://makerfoundry.app/designs/7",
            "javascript:alert(1)//https://makerfoundry.app/",
            "//makerfoundry.app/designs/7",
            "https://makerfoundry.app",
            "https://makerfoundry.app\\@evil.com/",
            "https://makerfoundry.app/designs/7 x",
            "https://makerfoundry.app/designs/7\n",
            "https://mаkerfoundry.app/designs/7",
        ] {
            let body = json!({"id": 7, "url": url}).to_string();
            assert_eq!(published(201, &body), Err(MfError::OffSite), "{url}");
        }
        let long = format!("https://makerfoundry.app/{}", "a".repeat(3000));
        assert_eq!(published(201, &json!({"id": 1, "url": long}).to_string()), Err(MfError::OffSite));
        for bad in
            ["", "not json", r#"{"url": "https://makerfoundry.app/d/1"}"#, r#"{"id": "7", "url": "https://makerfoundry.app/d"}"#, r#"{"id": 7}"#]
        {
            assert_eq!(published(201, bad), Err(MfError::NoAnswer), "{bad}");
        }
    }

    #[test]
    fn refusals_show_makerfoundrys_own_sentence() {
        let said = |status, msg: &str| published(status, &json!({"error": {"message": msg}}).to_string());
        assert_eq!(said(401, "Sign in again."), Err(MfError::Said(401, "Sign in again.".into())));
        assert_eq!(
            said(422, "That isn't a design MakerFoundry can read."),
            Err(MfError::Said(422, "That isn't a design MakerFoundry can read.".into()))
        );
        // One line, no controls, not endless.
        assert_eq!(said(429, "Too many\r\n\u{1b}[31m at once"), Err(MfError::Said(429, "Too many [31m at once".into())));
        let MfError::Said(_, long) = said(503, &"z".repeat(5000)).unwrap_err() else { panic!() };
        assert_eq!(long.chars().count(), 300);
        // Without a sentence: what the status means.
        assert_eq!(published(401, "<html>"), Err(MfError::SignIn));
        assert_eq!(published(413, ""), Err(MfError::TooLarge));
        assert_eq!(published(503, "{}"), Err(MfError::Said(503, "HTTP 503".into())));
        assert_eq!(MfError::Said(400, "Bad.".into()).to_string(), "MakerFoundry: Bad.");
    }
}
