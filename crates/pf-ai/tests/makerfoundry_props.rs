//! Untrusted input to the MakerFoundry requests (PF-11): whatever MakerFoundry answers, whatever
//! a draft's address says and whatever the person names a design, nothing panics, no page off
//! MakerFoundry's own site is ever given back to open, and refusals stay one short line.
// Integration tests: unwrapping and panicking on failure is fine here, unlike in shipped code (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use pf_ai::makerfoundry::{MfError, file_name, on_site, publish, published, refusal};
use proptest::prelude::*;
use serde_json::json;

/// Addresses close to MakerFoundry's: its host with things around it, or other hosts and schemes.
fn near_address() -> impl Strategy<Value = String> {
    let scheme = prop_oneof![Just("https://"), Just("http://"), Just("HTTPS://"), Just("//"), Just(""), Just("javascript:")];
    let host = prop_oneof![
        Just("makerfoundry.app".to_string()),
        Just("www.makerfoundry.app".to_string()),
        Just("makerfoundry.app.evil.com".to_string()),
        Just("evil.com".to_string()),
        Just("makerfoundry.app:443".to_string()),
        Just("user@makerfoundry.app".to_string()),
        Just("makerfoundry.app.".to_string()),
        Just("MakerFoundry.app".to_string()),
        Just("makerfoundry\u{430}.app".to_string()),
        ".{0,20}",
    ];
    let path = prop_oneof![Just("/designs/12/edit".to_string()), Just(String::new()), ".{0,40}", "[/?#@:\\\\ a-z0-9.%]{0,40}"];
    (scheme, host, path).prop_map(|(s, h, p)| format!("{s}{h}{p}"))
}

/// What `on_site` should say, checked the slow way through the address's parts.
fn really_on_site(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://makerfoundry.app") else { return false };
    url.len() <= 2048 && rest.starts_with('/') && url.bytes().all(|b| b.is_ascii_graphic() && b != b'\\')
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn only_makerfoundrys_own_pages_are_on_site(url in near_address()) {
        prop_assert_eq!(on_site(&url), really_on_site(&url), "{:?}", url);
    }

    #[test]
    fn any_answer_is_read_without_panicking_and_never_gives_a_page_elsewhere(
        status in prop_oneof![Just(200u16), Just(201), Just(400), Just(401), Just(413), Just(422), Just(429), Just(503), any::<u16>()],
        body in ".{0,200}",
    ) {
        if let Ok(p) = published(status, &body) {
            prop_assert!(on_site(&p.url));
        }
    }

    #[test]
    fn a_draft_is_given_back_only_on_site(id in any::<u64>(), url in near_address()) {
        let body = json!({ "id": id, "url": url, "kind": "laser", "format": "svg" }).to_string();
        match published(201, &body) {
            Ok(p) => {
                prop_assert!(really_on_site(&url));
                prop_assert_eq!(p.id, id);
                prop_assert_eq!(p.url, url);
            }
            Err(e) => {
                prop_assert!(!really_on_site(&url));
                prop_assert_eq!(e, MfError::OffSite);
            }
        }
    }

    #[test]
    fn a_refusal_is_one_short_line(status in 400u16..600, message in "(.|\n|\r|\u{1b}){0,600}") {
        let body = json!({ "error": { "message": message } }).to_string();
        let text = refusal(status, &body).to_string();
        prop_assert!(!text.chars().any(char::is_control), "{:?}", text);
        prop_assert!(text.chars().count() <= 300 + "MakerFoundry: ".len());
    }

    #[test]
    fn any_name_makes_a_tidy_file_name(title in ".{0,200}") {
        let name = file_name(&title, "svg");
        let stem = name.strip_suffix(".svg").unwrap();
        prop_assert!(!stem.is_empty() && stem.chars().count() <= 60, "{:?}", name);
        prop_assert!(stem.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'), "{:?}", name);
        prop_assert!(stem.chars().any(char::is_alphanumeric));
    }

    #[test]
    fn only_a_session_token_ever_leaves(token in ".{0,40}", len in 0usize..20_000_000) {
        if let Ok(r) = publish(&token, "a.svg", "PA==", len, "PatternFoundry") {
            let t = token.trim();
            prop_assert!(t.starts_with("sfs_") && t.len() > 4 && t.bytes().all(|b| b.is_ascii_graphic()));
            prop_assert!(len > 0 && len <= 8 * 1024 * 1024);
            prop_assert_eq!(r.url, "https://makerfoundry.app/api/v1/designs");
            let auth: Vec<_> = r.headers.iter().filter(|h| h.0 == "authorization").collect();
            prop_assert_eq!(auth.len(), 1);
            prop_assert_eq!(&auth[0].1, &format!("Bearer {t}"));
        }
    }
}
