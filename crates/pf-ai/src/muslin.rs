//! Check My Muslin: photos of a muslin (a toile, the test garment sewn in cheap fabric) tried on,
//! with the pattern it was cut from and a few answers about the fit, looked at by Claude, which
//! says what doesn't fit, why, and how to alter the pattern, and, with the user's leave, makes the
//! alterations through the same tools as Ask About Patterns ([`super::ask::tools`]). The check is
//! the first question of an Ask conversation ([`first_message`]) with its own instructions
//! ([`system_prompt`], [`request`]): the follow-up questions, the tool rounds and the replies go
//! as in [`super::ask`].
//!
//! Adapted from StitchFoundry's Check My Sew-out (`crates/sf-core/src/ai/sewout.rs`). The photos
//! show the user's body: they go with this request only, and nothing here keeps or logs them.

use serde_json::{Value, json};

use super::ask::{APP_GUIDE, FIND, MAX_PATTERN, RUN, request_with};
use super::{AiError, AiImage, HttpRequest, MAX_IMAGE_BASE64};

/// The most photos one check sends.
pub const MAX_PHOTOS: usize = 6;
/// The longest notes sent, in characters.
pub const MAX_NOTES: usize = 1_000;

/// What the user says about the fit that a photo can't show; empty fields are left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answers {
    /// The muslin's fabric (calico, an old sheet…).
    pub fabric: String,
    /// The fabric the garment will be made in.
    pub final_fabric: String,
    /// How it feels, each in the app's words ("tight across the bust").
    pub feels: Vec<String>,
    /// Anything else, in the user's words.
    pub notes: String,
}

impl Answers {
    fn text(&self) -> String {
        let mut lines = Vec::new();
        for (label, v) in [("Muslin fabric", &self.fabric), ("The garment will be made in", &self.final_fabric)] {
            let v = v.trim();
            if !v.is_empty() {
                lines.push(format!("{label}: {v}"));
            }
        }
        let feels: Vec<&str> = self.feels.iter().map(|h| h.trim()).filter(|h| !h.is_empty()).collect();
        lines.push(format!("How it feels: {}", if feels.is_empty() { "nothing I noticed".to_string() } else { feels.join("; ") }));
        let notes: String = self.notes.trim().chars().take(MAX_NOTES).collect();
        if !notes.is_empty() {
            lines.push(format!("My notes: {notes}"));
        }
        lines.join("\n")
    }
}

/// The instructions for a check and the conversation after it.
pub fn system_prompt() -> String {
    format!(
        "You look at photos of a muslin (a toile: a test garment sewn in cheap fabric) tried on, \
inside PatternFoundry, a sewing-pattern drafting app, for people who make their own clothes, \
from beginners to pattern makers. Say what doesn't fit, why, and how to alter the pattern, as an \
experienced fitter would.\n\n\
The first message holds the user's photos (the front, the side, the back, close-ups), then, when \
the pattern is open, a summary of it written by the app: its pieces and their sizes, the blocks \
drafted, which measurements are set (never their values) and any size range. It ends with what \
the user says about the fabric and how the muslin feels.\n\n\
How to answer:\n\
- Look before you judge. Read the drag lines and folds: where they start and where they point \
tell what is too tight (lines pulling toward the strain) or too loose (folds hanging from the \
excess). Look at whether the side seams hang straight down, the centre front and back are \
plumb, the waist and hem are level, the shoulder seam sits on the shoulder and the bust point \
sits over the bust. Name only what you can see, where it is (\"the folds from the armhole \
toward the bust\"), and say how sure you are. If the photos can't settle it (a baggy top over \
it, a mirror's angle, too far away), say which photo would: front, side and back, standing \
straight, arms down.\n\
- The photos show a person: talk about the garment and the fit, never about the body's looks, \
and never guess the person's measurements from the photos.\n\
- For each problem give the likely cause, then the fix: first the pattern alteration (a full \
or small bust adjustment, a forward or square shoulder, a swayback, a sloped or raised waist, \
taking in or letting out a seam, lengthening or shortening, moving a dart), with how much in \
millimetres (inches in brackets) as a starting point; then anything to try in the sewing \
(stitching, pressing, the muslin's grain). Start with what fits well, briefly; then the \
problems, the most important first; at most five. Plain text only: no headings, tables, bold or \
code, and \"- \" for list items.\n\
- Offer to make the alterations. Change the pattern only when the user agrees: then find the \
commands with {FIND} and run them with {RUN}; each change is shown to the user, who allows or \
declines it; a declined change comes back as an error, so don't try it again. Commands that \
only read (piece.list, piece.info, design.length, document.node and the like) run at once; the \
measurements are personal: read them (measure.info) only when an alteration needs them, and the \
user is asked first. A block drafted from the measurements follows them, so a change in a \
measurement (measure.set) may be the simplest fix. Most commands act on the selected objects, so \
select them first (select.set {{\"ids\": [12]}}). Objects are named by their ids (#12). Document \
positions are in points (1 mm = 2.835 pt) unless a command's parameters say mm. After changing \
it, say what changed and suggest sewing the altered part again in muslin before cutting good \
fabric; Edit › Undo takes each change back.\n\
- Without the pattern open, give the alterations to make by hand, and say that opening the \
pattern lets you make them.\n\
- For photos that aren't of a garment being fitted, say briefly that you can only look at \
muslins and fit.\n\n{APP_GUIDE}"
    )
}

/// The check as the first user message: the photos, then the pattern summary when the pattern
/// is open, then the answers.
pub fn first_message(photos: &[AiImage], pattern: Option<&str>, answers: &Answers) -> Result<Value, AiError> {
    if photos.is_empty() {
        return Err(AiError::NoPhoto);
    }
    if photos.iter().any(|p| p.data_base64.len() > MAX_IMAGE_BASE64) {
        return Err(AiError::LargeImage);
    }
    let image = |p: &AiImage| json!({ "type": "image", "source": { "type": "base64", "media_type": p.media_type, "data": p.data_base64 } });
    let mut blocks: Vec<Value> = Vec::new();
    let n = photos.len().min(MAX_PHOTOS);
    blocks.push(json!({ "type": "text", "text": format!("My muslin: {n} photo{}.", if n == 1 { "" } else { "s" }) }));
    blocks.extend(photos.iter().take(MAX_PHOTOS).map(image));
    if let Some(d) = pattern.map(str::trim).filter(|d| !d.is_empty()) {
        let d: String = d.chars().take(MAX_PATTERN).collect();
        blocks.push(json!({ "type": "text", "text": format!("The pattern it was cut from (from the app, not from the user):\n{d}") }));
    }
    blocks.push(json!({ "type": "text", "text": format!("{}\n\nWhat doesn't fit, and how do I alter the pattern?", answers.text()) }));
    Ok(json!({ "role": "user", "content": blocks }))
}

/// The HTTP request for a check's conversation `messages` (its first message from
/// [`first_message`]), with API key `key` (or a Foundry session token).
pub fn request(key: &str, messages: &[Value]) -> Result<HttpRequest, AiError> {
    request_with(key, system_prompt(), messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn photo(data: &str) -> AiImage {
        AiImage { media_type: "image/jpeg", data_base64: data.to_string() }
    }

    #[test]
    fn the_check_sends_the_photos_the_pattern_and_the_answers() {
        let answers = Answers {
            fabric: " calico ".into(),
            final_fabric: String::new(),
            feels: vec!["tight across the bust".into(), "  ".into()],
            notes: "folds below the back neck".into(),
        };
        let m = first_message(&[photo("AAAA"), photo("BBBB")], Some("2 pattern pieces"), &answers).unwrap();
        let c = m["content"].as_array().unwrap();
        assert_eq!(c[0]["text"], "My muslin: 2 photos.");
        assert_eq!(c[1]["source"], json!({"type": "base64", "media_type": "image/jpeg", "data": "AAAA"}));
        assert_eq!(c[2]["type"], "image");
        assert!(c[3]["text"].as_str().unwrap().ends_with("2 pattern pieces"));
        let text = c[4]["text"].as_str().unwrap();
        assert!(text.starts_with("Muslin fabric: calico\nHow it feels: tight across the bust\nMy notes: folds below the back neck"), "{text}");
        assert!(!text.contains("made in"), "an empty answer is left out");
        // Without the pattern, only the photos and the answers.
        let m = first_message(&[photo("AAAA")], None, &Answers::default()).unwrap();
        assert_eq!(m["content"].as_array().unwrap().len(), 3);
        assert!(m["content"][2]["text"].as_str().unwrap().contains("How it feels: nothing I noticed"));
        // Photos are needed, and not too large.
        assert_eq!(first_message(&[], None, &Answers::default()), Err(AiError::NoPhoto));
        assert_eq!(first_message(&[photo(&"A".repeat(MAX_IMAGE_BASE64 + 1))], None, &Answers::default()), Err(AiError::LargeImage));
        // At most six photos go.
        let many: Vec<AiImage> = (0..9).map(|_| photo("A")).collect();
        let m = first_message(&many, None, &Answers::default()).unwrap();
        assert_eq!(m["content"].as_array().unwrap().iter().filter(|b| b["type"] == "image").count(), MAX_PHOTOS);
    }

    #[test]
    fn the_request_has_the_fitting_instructions_and_the_tools() {
        let m = first_message(&[photo("AAAA")], None, &Answers::default()).unwrap();
        let http = request("sk-ant-test", &[m]).unwrap();
        let body: Value = serde_json::from_str(&http.body).unwrap();
        let system = body["system"].as_str().unwrap();
        assert!(
            system.contains("drag lines") && system.contains("never guess the person's measurements") && system.contains("measure.info"),
            "{system}"
        );
        assert!(system.contains("Draft Bodice Block"), "the app's menus");
        let tools: Vec<&str> = body["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(tools, [FIND, RUN]);
        assert_ne!(system, super::super::ask::system_prompt());
    }
}
