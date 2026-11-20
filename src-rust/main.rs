//! Standalone Tezbar process executable for the Lorem Ipsum extension.
//!
//! Rust port of the original extension commands and the newer HTML/Markdown
//! document generators. It serves protocol 1
//! JSONL over stdio through the pinned `tezbar-extension-sdk` and only uses the
//! typed host methods granted in `extension.json`. No JavaScript runtime.
mod documents;
mod lorem;

use serde_json::{json, Value};
use tezbar_extension_sdk::{BoxFuture, CoreResult, Extension, ExtensionContext};

pub const ID: &str = "com.tezbar.native.lorem-ipsum";
pub const COMMANDS: &[&str] = &[
    "paragraphs",
    "sentences",
    "words",
    "markdown",
    "html",
    "ai-generate",
];
/// The host rejects clipboard/paste text above 1 MiB (`process_host::string`).
pub const HOST_TEXT_LIMIT: usize = 1024 * 1024;

pub const COPIED: &str = "Copied to clipboard! 📋";
pub const PASTED: &str = "Pasted to active app! 📝";
pub const PASTED_AND_COPIED: &str = "Pasted to active app and copied to clipboard! 📋";
pub const NO_AI: &str = "You don't have access to AI.";

/// The `action` preference (`Default Action`); missing means the manifest default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Clipboard,
    Paste,
    PasteAndCopy,
}

pub fn action(preferences: &Value) -> Result<Action, String> {
    match &preferences["action"] {
        Value::Null => Ok(Action::Clipboard),
        Value::String(value) => match value.as_str() {
            "clipboard" => Ok(Action::Clipboard),
            "paste" => Ok(Action::Paste),
            "pasteAndCopy" => Ok(Action::PasteAndCopy),
            other => Err(format!("Unknown Default Action preference: {other}")),
        },
        _ => Err("Default Action preference must be text".into()),
    }
}

/// Optional text argument; missing/null is "not supplied" like `props?.arguments`.
pub fn text_argument<'a>(arguments: &'a Value, name: &str) -> Result<Option<&'a str>, String> {
    match &arguments[name] {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text.as_str())),
        _ => Err(format!("Argument {name} must be text")),
    }
}

/// Local text generation for the three non-AI commands.
pub fn generate(
    command: &str,
    arguments: &Value,
    generator: &mut lorem::Generator<impl lorem::Random>,
) -> Result<String, String> {
    if matches!(command, "markdown" | "html") {
        return Ok(documents::generate(generator, command == "html"));
    }
    let argument = match command {
        "paragraphs" => "numberOfParagraphs",
        "sentences" => "numberOfSentences",
        "words" => "numberOfWords",
        _ => return Err(format!("Unknown command: {command}")),
    };
    let count = lorem::safe_number(text_argument(arguments, argument)?)?;
    Ok(match command {
        "paragraphs" => generator.paragraphs(count),
        "sentences" => generator.sentences(count),
        _ => generator.words(count),
    })
}

/// `props?.arguments.topic || undefined`: only the empty string is falsy.
pub fn topic(arguments: &Value) -> Result<Option<&str>, String> {
    Ok(text_argument(arguments, "topic")?.filter(|t| !t.is_empty()))
}

fn granted(ctx: &ExtensionContext, method: &str) -> bool {
    let capability = method.strip_prefix("host.").unwrap_or(method);
    let group = capability.split('.').next().unwrap_or(capability);
    [method, capability, group]
        .iter()
        .any(|c| ctx.granted_capabilities.contains(*c))
}

/// `produceOutput` of `src/utils.tsx`.
async fn produce_output(ctx: &ExtensionContext, content: &str, action: Action) -> CoreResult {
    if content.len() > HOST_TEXT_LIMIT {
        return Err(format!(
            "Generated text is {} bytes; Tezbar accepts at most {HOST_TEXT_LIMIT} bytes. Try a smaller number.",
            content.len()
        ));
    }
    let message = match action {
        Action::Clipboard => {
            tezbar_extension_sdk::copy(ctx, content).await?;
            // closeMainWindow: after the copy so a failed copy stays visible.
            if let Err(error) = ctx
                .host("host.window.visibility", json!({"visible": false}))
                .await
            {
                eprintln!("lorem-ipsum: could not hide Tezbar after copying: {error}");
            }
            COPIED
        }
        // The host writes the clipboard, hides Tezbar, restores focus, sends Cmd/Ctrl+V.
        Action::Paste => {
            ctx.host("host.clipboard.pasteText", json!({"text": content}))
                .await?;
            PASTED
        }
        Action::PasteAndCopy => {
            ctx.host("host.clipboard.pasteText", json!({"text": content}))
                .await?;
            tezbar_extension_sdk::copy(ctx, content).await?;
            PASTED_AND_COPIED
        }
    };
    Ok(tezbar_extension_sdk::no_view(message))
}

/// `AICommand` of `src/ai-generate.tsx`.
async fn ai_generate(ctx: &ExtensionContext, arguments: &Value, action: Action) -> CoreResult {
    if !granted(ctx, "host.ai.ask") {
        return Err(NO_AI.into());
    }
    let topic = topic(arguments)?;
    let prompt = lorem::ai_prompt(topic);
    // Raycast model/creativity are forwarded for provenance; Tezbar answers with
    // the user's configured AI provider.
    let response = ctx
        .host(
            "host.ai.ask",
            json!({"prompt": prompt, "options": {"model": "OpenAI_GPT-4o_mini", "creativity": "none"}}),
        )
        .await?;
    let output = response
        .as_str()
        .ok_or("AI returned a non-text response")?
        .trim();
    if output.is_empty() {
        return Err("AI returned an empty response".into());
    }
    produce_output(ctx, output, action).await
}

pub async fn run_command(
    ctx: &ExtensionContext,
    command: &str,
    payload: &Value,
    generator: &mut lorem::Generator<impl lorem::Random>,
) -> CoreResult {
    let action = action(&payload["preferences"])?;
    let arguments = &payload["argumentValues"];
    if command == "ai-generate" {
        return ai_generate(ctx, arguments, action).await;
    }
    let content = generate(command, arguments, generator)?;
    produce_output(ctx, &content, action).await
}

struct Adapter;

impl Extension for Adapter {
    fn id(&self) -> &'static str {
        ID
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn commands(&self) -> &'static [&'static str] {
        COMMANDS
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        operation: &'a str,
        payload: &'a Value,
        state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            match operation {
                "extension:dispose-session" => {
                    *state = json!({});
                    Ok(tezbar_extension_sdk::unchanged())
                }
                _ if !COMMANDS.contains(&command) => Err(format!("Unknown command: {command}")),
                "extension:run-command" => {
                    let mut generator = lorem::Generator::new(fastrand::Rng::new());
                    run_command(ctx, command, payload, &mut generator).await
                }
                // Every command is no-view: there is no view to search, refresh or act on.
                _ => Err(format!(
                    "Unsupported operation for no-view command: {operation}"
                )),
            }
        })
    }
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("extension runtime");
    let result = runtime.block_on(tezbar_extension_sdk::serve(Adapter));
    runtime.shutdown_timeout(std::time::Duration::from_millis(500));
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_preference_values_are_preserved() {
        assert_eq!(action(&json!({})), Ok(Action::Clipboard));
        assert_eq!(action(&json!({"action": null})), Ok(Action::Clipboard));
        assert_eq!(
            action(&json!({"action": "clipboard"})),
            Ok(Action::Clipboard)
        );
        assert_eq!(action(&json!({"action": "paste"})), Ok(Action::Paste));
        assert_eq!(
            action(&json!({"action": "pasteAndCopy"})),
            Ok(Action::PasteAndCopy)
        );
        assert!(action(&json!({"action": "copy"})).is_err());
        assert!(action(&json!({"action": 1})).is_err());
    }

    #[test]
    fn commands_read_their_original_argument_names() {
        let mut g = lorem::Generator::new(fastrand::Rng::with_seed(3));
        let words = generate("words", &json!({"numberOfWords": "6"}), &mut g).unwrap();
        assert_eq!(words.split(' ').count(), 6);
        let sentences = generate("sentences", &json!({"numberOfSentences": "3"}), &mut g).unwrap();
        assert_eq!(sentences.matches('.').count(), 3);
        let paragraphs =
            generate("paragraphs", &json!({"numberOfParagraphs": "4"}), &mut g).unwrap();
        assert_eq!(paragraphs.split("\r\n\r\n").count(), 4);
        // Default is one item when the optional argument is absent.
        assert_eq!(
            generate("paragraphs", &json!({}), &mut g)
                .unwrap()
                .split("\r\n\r\n")
                .count(),
            1
        );
        assert_eq!(
            generate("sentences", &json!({"numberOfSentences": ""}), &mut g)
                .unwrap()
                .matches('.')
                .count(),
            1
        );
        // A foreign argument name is ignored, as in the original.
        assert_eq!(
            generate("words", &json!({"numberOfParagraphs": "9"}), &mut g)
                .unwrap()
                .split(' ')
                .count(),
            1
        );
        assert!(generate("words", &json!({"numberOfWords": "1001"}), &mut g).is_err());
        assert!(generate("words", &json!({"numberOfWords": 5}), &mut g).is_err());
        assert!(generate("ai-generate", &json!({}), &mut g).is_err());
    }

    #[test]
    fn topic_matches_falsy_check() {
        assert_eq!(topic(&json!({})), Ok(None));
        assert_eq!(topic(&json!({"topic": ""})), Ok(None));
        assert_eq!(topic(&json!({"topic": "   "})), Ok(Some("   ")));
        assert_eq!(topic(&json!({"topic": "Rust"})), Ok(Some("Rust")));
    }

    #[test]
    fn upstream_toast_messages() {
        assert_eq!(COPIED, "Copied to clipboard! \u{1F4CB}");
        assert_eq!(PASTED, "Pasted to active app! \u{1F4DD}");
        assert_eq!(
            PASTED_AND_COPIED,
            "Pasted to active app and copied to clipboard! \u{1F4CB}"
        );
    }
}
