//! PatternFoundry's AI help (PF-8): Ask About Patterns ([`ask`]) and the Foundry Account that can
//! pay for it ([`account`]). Requests go out through the host's `http_post` service (desktop, on a
//! thread of their own) or `http_post_async` (the web, the browser's fetch), with the user's own
//! Anthropic API key or a Foundry session paid in credits; the windows pick the answers up on a
//! later frame.
//!
//! Adapted from StitchFoundry's AI windows (`crates/ui-egui/src/embroidery/{ask,account,prompt}.rs`
//! in github.com/Stitchwright/StitchFoundry), as `docs/start-here.md` plans.

use std::sync::mpsc::{Receiver, channel};

use pf_ai::{AiError, HttpRequest, cloud};

use crate::VectorcraftApp;

pub mod account;
pub mod ask;

/// The secret the API key is kept as.
pub const KEY_NAME: &str = "anthropic-api-key";
/// Where to get a key.
pub const KEY_URL: &str = "https://console.anthropic.com/settings/keys";

/// The AI windows between frames. Not saved: keys and sessions are kept by the host's secret
/// store.
#[derive(Default)]
pub struct AiState {
    /// The user's own Anthropic API key as typed or loaded. Never in a dialog's fields, which
    /// agents can read.
    pub key: String,
    /// Whether the secret store was read for the key.
    loaded: bool,
    /// The time of the last frame drawn (egui time, seconds).
    pub now: f64,
    pub ask: ask::AskState,
    pub account: account::AccountState,
}

/// Whether this build can send AI requests.
pub fn available(app: &VectorcraftApp) -> bool {
    app.services.http_post.is_some() || app.services.http_post_async.is_some()
}

/// Read the kept API key once.
pub(crate) fn load_key(app: &mut VectorcraftApp) {
    if !app.ai.loaded {
        app.ai.loaded = true;
        if let Some(k) = app.services.secrets.as_ref().and_then(|s| s.load(KEY_NAME)) {
            app.ai.key = k;
        }
    }
}

/// Keep the API key (`None`: forget it).
pub(crate) fn keep_key(store: Option<&dyn crate::SecretStore>, key: Option<&str>) {
    if let Some(Err(e)) = store.map(|s| s.save(KEY_NAME, key)) {
        log::warn!("Ask About Patterns: the API key: {e}");
    }
}

/// Sends AI request `http` (built with [`account::key_to_send`]): to Anthropic with the user's
/// own key, or, when paying with Foundry credits, to `path` on the Foundry service with `extra`
/// headers, its refusals read as the service's (sign in, credits).
pub(crate) fn post_ai<T: Send + 'static>(
    app: &VectorcraftApp,
    http: HttpRequest,
    thread: &str,
    path: &str,
    extra: Vec<(&'static str, String)>,
    parse: fn(u16, &str) -> Result<T, AiError>,
) -> Result<Receiver<Result<T, AiError>>, String> {
    let credits = account::paying_with_credits(app);
    let http = match account::server(app).filter(|_| credits) {
        Some(server) => cloud::through_cloud(http, &server, path, extra).map_err(|e| e.to_string())?,
        None => http,
    };
    post_plain(app, http, thread, move |status, body| {
        if credits && matches!(status, 401 | 402 | 403 | 503) { Err(cloud::status_error(status, body)) } else { parse(status, body) }
    })
}

/// Sends `http` with the host's service and reads the answer with `parse`: on the desktop on a
/// thread named `thread`, on the web through the browser. The result arrives on the receiver;
/// nothing waits for it once the receiver is dropped.
pub(crate) fn post_plain<T: Send + 'static, F: Fn(u16, &str) -> Result<T, AiError> + Send + 'static>(
    app: &VectorcraftApp,
    http: HttpRequest,
    thread: &str,
    parse: F,
) -> Result<Receiver<Result<T, AiError>>, String> {
    let (tx, rx) = channel();
    if let Some(post) = app.services.http_post.clone() {
        std::thread::Builder::new()
            .name(thread.into())
            .spawn(move || {
                let r = vectorcraft_engine::guard::catch_panic(|| post(&http))
                    .unwrap_or_else(|msg| Err(format!("internal error: {msg} (please report this bug)")))
                    .map_err(AiError::Network)
                    .and_then(|(status, body)| parse(status, &body));
                let _ = tx.send(r); // (nothing may wait any more)
            })
            .map_err(|e| format!("can't send the request: {e}"))?;
    } else if let Some(post) = app.services.http_post_async.clone() {
        post(
            http,
            Box::new(move |r| {
                let _ = tx.send(r.map_err(AiError::Network).and_then(|(status, body)| parse(status, &body))); // (nothing may wait any more)
            }),
        );
    } else {
        return Err("this needs a connection to the AI service, which this build doesn't have".into());
    }
    Ok(rx)
}

/// Each frame: take in the answers that arrived and draw the open windows.
pub fn frame(app: &mut VectorcraftApp, ctx: &egui::Context) {
    app.ai.now = ctx.input(|i| i.time);
    ask::poll(app, ctx);
    account::poll(app, ctx);
    ask::window(app, ctx);
    account::window(app, ctx);
}

#[cfg(test)]
mod tests;
