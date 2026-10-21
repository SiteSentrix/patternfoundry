//! Ask About Patterns: questions about sewing patterns or about using PatternFoundry, answered by
//! Claude, which can also look at the pattern open now and, with the user's leave, change it by
//! running PatternFoundry's commands (its two tools, [`tools`]): draft blocks from the
//! measurements, change the design, make pattern pieces, grade them. No network code lives here:
//! the app sends the [`HttpRequest`] that [`request`] builds, reads the answer with
//! [`parse_reply`], runs the commands asked for and sends their results back ([`tool_results`])
//! until Claude has answered.
//!
//! The conversation is kept as the API's own messages and only ever added to: replies go back
//! exactly as they came (thinking blocks included), and the pattern summary rides in each
//! question rather than in the instructions, which stay the same for the whole conversation.
//!
//! Adapted from StitchFoundry's Ask About Embroidery (`crates/sf-core/src/ai/ask.rs`).

use serde_json::{Value, json};

use super::{API_URL, API_VERSION, AiError, FALLBACK_BETA, HttpRequest, MODEL, check_status, clean_key, reply_text};

/// The most tokens one reply (thinking included) may use (the Foundry service's limit).
pub const MAX_TOKENS: u32 = 16_000;
/// The longest question sent, in characters.
pub const MAX_QUESTION: usize = 2_000;
/// The most questions one conversation holds (then: New Conversation).
pub const MAX_QUESTIONS: usize = 30;
/// The most requests one question may take (each command round is one): drafting a garment
/// takes many steps.
pub const MAX_ROUNDS: usize = 24;
/// The longest pattern summary sent, in characters.
pub const MAX_PATTERN: usize = 6_000;
/// The longest command result sent back, in characters (the start of it).
pub const MAX_RESULT: usize = 20_000;

/// The tool that searches the commands.
pub const FIND: &str = "find_commands";
/// The tool that runs one.
pub const RUN: &str = "run_command";

/// How PatternFoundry is laid out, so answers can name the right menu items.
pub(crate) const APP_GUIDE: &str = "\
PatternFoundry is a desktop and web app for drafting sewing patterns, built on a vector drawing \
app. Where things are:
- Pattern menu: Draft Bodice Block, Draft Skirt Block, Draft Sleeve Block, Draft Trouser Block \
(drafted from the measurements, Aldrich-style metric blocks); Rotate Dart, Split at Points, Make \
Facing, Gather, Draft Collar, Draft Stand Collar, Draft Shirt Collar, In-Seam Pocket, Welt \
Pocket; Make Pattern Piece and Release Pattern Piece; Grade Nest and Clear Graded Nest.
- Panels (Pattern and Window menus): Measurements (the body measurements in mm, formulas over \
them, and the size chart's sizes), Pattern Pieces (each piece's name, cut count, fabric and seam \
allowance), Grading (the size range, the base size and each point's grade rule).
- A pattern piece is a closed path made a piece: it gets a cutting line drawn at its seam \
allowance, notches, a grainline and a label. Blocks redraw themselves when the measurements \
change.
- File › Print… prints at 100% with Pattern Marks (a 10 cm and a 1 in test square on every page, \
tiles named A1, B2… with overlap lines), on home printer pages or copy-shop paper (A0, A1, the \
36 in roll). File › Export › Export As…: PDF, SVG, DXF-AAMA (cutting room), SVG for cutting \
(laser and blade cutters), PDF for projector (one page at 100% with calibration grids).
- Edit › Undo undoes any step. Help › PatternFoundry Help… (F1) opens the user guide.";

/// The instructions, the same for every request of a conversation.
pub fn system_prompt() -> String {
    format!(
        "You answer questions inside PatternFoundry, a sewing-pattern drafting app, for people who \
sew and make their own patterns, from beginners to pattern makers. Answer what was asked, in \
plain words, as briefly as the question allows: a few sentences, or a short list of steps. Write \
plain text only: no headings, tables, bold or code, and \"- \" for list items. Name \
PatternFoundry's menu items exactly as listed below when one does what the user needs; when you \
are not sure the app has something, say so rather than inventing a menu item. Give measurements \
in millimetres or centimetres with inches in brackets. Say so when a change affects the fit and \
suggest a muslin (toile) before cutting good fabric. For questions that have nothing to do with \
sewing, patterns or this app, say briefly that you can only help with sewing patterns.\n\n\
Each question may start with a summary of the pattern open now, written by the app; objects are \
named by their ids (#12). You can look at the pattern and change it with your tools: {FIND} \
finds PatternFoundry's commands and how to call them, and {RUN} runs one. Commands that only \
read (piece.list, piece.info, grade.info, document.node and the like) run at once. The user's \
body measurements are personal: read them (measure.info) only when the task needs them, and the \
user is asked first. Change the pattern only when the user asks for a change or agrees to one \
you suggested: each change is shown to the user, who allows or declines it; a declined change \
comes back as an error, so don't try it again, and say what you would have done instead. To draft \
a garment, set the measurements it needs (block.measurements lists them), draft the block, change \
the design with the design commands, then make the outlines pattern pieces. Most commands act on \
the selected objects, so select them first (select.set {{\"ids\": [12]}}). Document positions are \
in points (1 mm = 2.835 pt); measurements, allowances and design commands take mm unless their \
parameters say otherwise. After changing the pattern, say in one or two sentences what changed; \
Edit › Undo takes each change back.\n\n{APP_GUIDE}"
    )
}

/// The two tools: finding commands and running one.
pub fn tools() -> Value {
    json!([
        {
            "name": FIND,
            "description": "Search PatternFoundry's commands by a few words, such as \"dart\", \"seam allowance\", \"skirt block\", \"measurement\", \"grade\" or \"notch\". Returns up to 20 commands, best first, each with its id, its label and menu, whether it can run now, and its parameters as \"{key, key?: …} what it does → result\" (a ? marks an optional key).",
            "input_schema": {
                "type": "object",
                "properties": { "query": { "type": "string", "description": "Words to look for in command ids, labels and menus" } },
                "required": ["query"],
            },
        },
        {
            "name": RUN,
            "description": "Run one PatternFoundry command, by its id from find_commands, with its parameters. Commands that only read run at once and return their result as JSON. Commands that change the pattern, and reading the user's measurements, wait until the user allows them; one the user declines returns an error. File, export, preference and window commands can't be run from here.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The command id, such as block.skirt" },
                    "params": { "type": "object", "description": "The command's parameters (an empty object when it takes none)" },
                },
                "required": ["command"],
            },
        },
    ])
}

/// A question as the user message it is sent as: the pattern summary (if the user lets it go
/// along), then the question. Trimmed and checked first.
pub fn question(question: &str, pattern: Option<&str>) -> Result<Value, AiError> {
    let q = question.trim();
    if q.is_empty() {
        return Err(AiError::NoQuestion);
    }
    if q.chars().count() > MAX_QUESTION {
        return Err(AiError::LongQuestion);
    }
    let mut blocks = Vec::new();
    if let Some(d) = pattern.map(str::trim).filter(|d| !d.is_empty()) {
        let d: String = d.chars().take(MAX_PATTERN).collect();
        blocks.push(json!({ "type": "text", "text": format!("The pattern open now (from the app, not from the user):\n{d}") }));
    }
    blocks.push(json!({ "type": "text", "text": q }));
    Ok(json!({ "role": "user", "content": blocks }))
}

/// A command's result for Claude: `Ok` text or the error why it didn't run.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolResult {
    pub id: String,
    pub result: Result<String, String>,
}

/// The results of every tool call of one reply, as the one user message they go back in.
pub fn tool_results(results: &[ToolResult]) -> Value {
    let blocks: Vec<Value> = results
        .iter()
        .map(|r| match &r.result {
            Ok(text) => {
                let text: String = text.chars().take(MAX_RESULT).collect();
                json!({ "type": "tool_result", "tool_use_id": r.id, "content": text })
            }
            Err(e) => json!({ "type": "tool_result", "tool_use_id": r.id, "content": e, "is_error": true }),
        })
        .collect();
    json!({ "role": "user", "content": blocks })
}

/// The HTTP request for the conversation `messages` (the API's messages, oldest first, ending
/// with a question or tool results), with API key `key` (or a Foundry session token, which
/// [`super::cloud::through_cloud`] moves into the service's own header).
pub fn request(key: &str, messages: &[Value]) -> Result<HttpRequest, AiError> {
    let key = clean_key(key)?;
    let body = json!({
        "model": MODEL,
        "max_tokens": MAX_TOKENS,
        "fallbacks": "default",
        "output_config": { "effort": "medium" },
        "system": system_prompt(),
        "tools": tools(),
        "messages": messages,
    });
    Ok(HttpRequest {
        url: API_URL.to_string(),
        headers: vec![
            ("content-type", "application/json".to_string()),
            ("x-api-key", key.to_string()),
            ("anthropic-version", API_VERSION.to_string()),
            ("anthropic-beta", FALLBACK_BETA.to_string()),
        ],
        body: body.to_string(),
    })
}

/// A tool call in a reply.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// A reply: the assistant message to keep, its text and the tool calls to answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    /// The reply as the assistant message added to the conversation, unchanged.
    pub message: Value,
    /// Its text blocks joined (cut-off text ends in "…").
    pub text: String,
    /// The tools to run before Claude can go on; empty once it has answered.
    pub calls: Vec<ToolCall>,
    /// The model that answered (another one when the request fell back).
    pub model: String,
}

/// The reply with HTTP status `status` and body `body`.
pub fn parse_reply(status: u16, body: &str) -> Result<Reply, AiError> {
    check_status(status, body)?;
    let v: Value = serde_json::from_str(body).map_err(|_| AiError::NoAnswer)?;
    let stop = v.get("stop_reason").and_then(Value::as_str);
    if stop == Some("refusal") {
        return Err(AiError::RefusedAnswer);
    }
    let content = v.get("content").filter(|c| c.is_array()).cloned().ok_or(AiError::NoAnswer)?;
    let calls: Vec<ToolCall> = if stop == Some("tool_use") {
        content
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"))
            .filter_map(|b| {
                Some(ToolCall {
                    id: b.get("id")?.as_str()?.to_string(),
                    name: b.get("name")?.as_str()?.to_string(),
                    input: b.get("input").cloned().unwrap_or_else(|| json!({})),
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut text = reply_text(&v).trim().to_string();
    if text.is_empty() && calls.is_empty() {
        return Err(AiError::NoAnswer);
    }
    if stop == Some("max_tokens") && !text.is_empty() {
        text.push('…');
    }
    let model = v.get("model").and_then(Value::as_str).unwrap_or(MODEL).to_string();
    Ok(Reply { message: json!({ "role": "assistant", "content": content }), text, calls, model })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(stop: &str, content: Value) -> String {
        json!({"type": "message", "model": MODEL, "content": content, "stop_reason": stop}).to_string()
    }

    #[test]
    fn the_request_holds_the_tools_the_instructions_and_the_conversation() {
        let q = question("  How much ease for a fitted bodice?  ", Some("2 pattern pieces")).unwrap();
        let http = request(" sk-ant-test ", std::slice::from_ref(&q)).unwrap();
        assert_eq!(http.url, API_URL);
        let header = |k: &str| http.headers.iter().find(|h| h.0 == k).map(|h| h.1.as_str());
        assert_eq!(header("x-api-key"), Some("sk-ant-test"));
        assert_eq!(header("anthropic-beta"), Some(FALLBACK_BETA));
        let body: Value = serde_json::from_str(&http.body).unwrap();
        assert_eq!(body["model"], MODEL);
        assert_eq!(body["max_tokens"], MAX_TOKENS);
        assert_eq!(body["fallbacks"], "default");
        assert_eq!(body["output_config"]["effort"], "medium");
        // only what the Foundry service accepts
        for k in body.as_object().unwrap().keys() {
            assert!(
                ["model", "max_tokens", "system", "messages", "tools", "tool_choice", "output_config", "fallbacks", "thinking"].contains(&k.as_str()),
                "{k}"
            );
        }
        let names: Vec<&str> = body["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, [FIND, RUN]);
        let system = body["system"].as_str().unwrap();
        assert!(system.contains("Draft Skirt Block") && system.contains("plain text only") && system.contains("measure.info"), "{system}");
        // the pattern rides in the question, before it
        assert_eq!(
            body["messages"],
            json!([{"role": "user", "content": [
                {"type": "text", "text": "The pattern open now (from the app, not from the user):\n2 pattern pieces"},
                {"type": "text", "text": "How much ease for a fitted bodice?"},
            ]}])
        );
        // the instructions don't change with the pattern: the conversation is only added to
        assert_eq!(system, system_prompt());
    }

    #[test]
    fn questions_and_keys_are_checked_before_sending() {
        assert_eq!(question("  ", None), Err(AiError::NoQuestion));
        assert_eq!(question(&"x".repeat(MAX_QUESTION + 1), None), Err(AiError::LongQuestion));
        assert_eq!(question("q", Some("  ")).unwrap()["content"].as_array().unwrap().len(), 1, "an empty summary is left out");
        assert_eq!(request(" ", &[]), Err(AiError::NoKey));
        assert_eq!(request("sk ant", &[]), Err(AiError::BadKey));
    }

    #[test]
    fn a_reply_with_tool_calls_is_kept_whole_and_its_calls_listed() {
        let content = json!([
            {"type": "thinking", "thinking": "", "signature": "sig"},
            {"type": "text", "text": "Let me look."},
            {"type": "tool_use", "id": "toolu_1", "name": RUN, "input": {"command": "piece.list", "params": {}}},
            {"type": "tool_use", "id": "toolu_2", "name": FIND, "input": {"query": "skirt block"}},
        ]);
        let r = parse_reply(200, &reply("tool_use", content.clone())).unwrap();
        assert_eq!(r.message, json!({"role": "assistant", "content": content}), "thinking blocks go back unchanged");
        assert_eq!(r.text, "Let me look.");
        assert_eq!(r.calls.len(), 2);
        assert_eq!(r.calls[0], ToolCall { id: "toolu_1".into(), name: RUN.into(), input: json!({"command": "piece.list", "params": {}}) });
        // their results go back in one message, an error marked as one
        let m = tool_results(&[
            ToolResult { id: "toolu_1".into(), result: Ok("x".repeat(MAX_RESULT + 10)) },
            ToolResult { id: "toolu_2".into(), result: Err("the user declined this change".into()) },
        ]);
        assert_eq!(m["role"], "user");
        assert_eq!(m["content"][0]["content"].as_str().unwrap().len(), MAX_RESULT);
        assert_eq!(
            m["content"][1],
            json!({"type": "tool_result", "tool_use_id": "toolu_2", "content": "the user declined this change", "is_error": true})
        );
    }

    #[test]
    fn answers_and_their_errors() {
        let a = parse_reply(200, &reply("end_turn", json!([{"type": "text", "text": "  Add 10 cm of ease at the bust.  "}]))).unwrap();
        assert_eq!((a.text.as_str(), a.calls.len(), a.model.as_str()), ("Add 10 cm of ease at the bust.", 0, MODEL));
        assert_eq!(parse_reply(200, &reply("max_tokens", json!([{"type": "text", "text": "First,"}]))).unwrap().text, "First,…");
        assert_eq!(parse_reply(200, &reply("end_turn", json!([]))), Err(AiError::NoAnswer));
        assert_eq!(parse_reply(200, &reply("refusal", json!([]))), Err(AiError::RefusedAnswer));
        assert_eq!(parse_reply(200, "not json"), Err(AiError::NoAnswer));
        assert_eq!(parse_reply(429, ""), Err(AiError::Busy));
        assert!(matches!(parse_reply(401, r#"{"error": {"message": "invalid x-api-key"}}"#), Err(AiError::Key(m)) if m == "invalid x-api-key"));
    }
}
