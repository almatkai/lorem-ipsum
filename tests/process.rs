//! Drives the real `tezbar-lorem-ipsum` executable over stdio JSONL pipes. The test
//! plays the Tezbar host: it answers typed `host.*` reverse RPCs from a script and
//! records them. No real clipboard, paste, AI provider or window is touched.
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};

const BIN: &str = env!("CARGO_BIN_EXE_tezbar-lorem-ipsum");
const ID: &str = "com.tezbar.native.lorem-ipsum";
const ALL: &[&str] = &[
    "clipboard.write",
    "clipboard.pasteText",
    "window.visibility",
    "ai.ask",
];
const INVALID: &str = "Please enter a valid integer number, no more than 1000";

type Reply = Box<dyn FnMut(&str, &Value) -> Result<Value, String>>;

struct Peer {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    calls: Vec<(String, Value)>,
    reply: Reply,
    sequence: u64,
    _dir: tempfile::TempDir,
}

fn ok() -> Reply {
    Box::new(|_, _| Ok(Value::Null))
}

impl Peer {
    async fn start(grants: &[&str], reply: Reply) -> Self {
        let mut child = Command::new(BIN)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        let dir = tempfile::tempdir().unwrap();
        let mut peer = Self {
            child,
            input,
            output,
            calls: vec![],
            reply,
            sequence: 1,
            _dir: dir,
        };
        let hello = peer.frame().await;
        assert_eq!(hello["type"], "hello");
        assert_eq!(hello["id"], ID);
        assert_eq!(hello["protocol"], 1);
        assert_eq!(hello["version"], env!("CARGO_PKG_VERSION"));
        let root = peer._dir.path().to_path_buf();
        let init = peer
            .request(
                "initialize",
                json!({"protocol":1,"extensionId":ID,
                    "platform":if cfg!(windows) {"Windows"} else {"macOS"},
                    "packageDir":env!("CARGO_MANIFEST_DIR"),
                    "dataDir":root.join("data"),"tempDir":root.join("temp"),
                    "grantedCapabilities":grants,"reason":"command","migrationPaths":{}}),
            )
            .await
            .unwrap();
        assert_eq!(init["ready"], true);
        peer
    }
    async fn send(&mut self, value: Value) {
        let mut bytes = serde_json::to_vec(&value).unwrap();
        bytes.push(b'\n');
        let input = self.input.as_mut().unwrap();
        input.write_all(&bytes).await.unwrap();
        input.flush().await.unwrap();
    }
    async fn frame(&mut self) -> Value {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(10), self.output.read_line(&mut line))
            .await
            .expect("protocol timeout")
            .unwrap();
        assert!(n > 0, "unexpected EOF");
        assert!(
            line.ends_with('\n') && !line.ends_with("\r\n"),
            "LF framing"
        );
        serde_json::from_str(&line).unwrap()
    }
    /// Sends a host request and services reverse RPCs until its response.
    async fn request(&mut self, method: &str, params: Value) -> Result<Value, Value> {
        let id = format!("h:00000000-0000-4000-8000-{:012x}", self.sequence);
        self.sequence += 1;
        self.send(json!({"type":"request","id":id,"method":method,"params":params}))
            .await;
        loop {
            let frame = self.frame().await;
            match frame["type"].as_str() {
                Some("response") if frame["id"] == id.as_str() => {
                    return match frame.get("error") {
                        Some(error) => Err(error.clone()),
                        None => Ok(frame["result"].clone()),
                    };
                }
                Some("request") => {
                    let method = frame["method"].as_str().unwrap().to_owned();
                    assert!(frame["id"].as_str().unwrap().starts_with("e:"));
                    assert!(method.starts_with("host."), "non-typed RPC {method}");
                    let params = frame["params"].clone();
                    let result = (self.reply)(&method, &params);
                    self.calls.push((method, params));
                    let reply = match result {
                        Ok(value) => json!({"type":"response","id":frame["id"],"result":value}),
                        Err(message) => json!({"type":"response","id":frame["id"],
                            "error":{"code":"host_error","message":message}}),
                    };
                    self.send(reply).await;
                }
                Some("event") => {}
                other => panic!("unexpected frame {other:?}: {frame}"),
            }
        }
    }
    async fn open(
        &mut self,
        session: &str,
        command: &str,
        preferences: Value,
        arguments: Value,
    ) -> Result<Value, Value> {
        let result = self
            .request(
                "session.open",
                json!({"sessionId":session,"commandName":command,
                    "preferences":preferences,"argumentValues":arguments}),
            )
            .await;
        if result.is_ok() {
            self.request("session.dispose", json!({"sessionId":session}))
                .await
                .unwrap();
        }
        result
    }
    fn methods(&self) -> Vec<&str> {
        self.calls.iter().map(|(m, _)| m.as_str()).collect()
    }
    fn take(&mut self) -> Vec<(String, Value)> {
        std::mem::take(&mut self.calls)
    }
    async fn shutdown(mut self) {
        self.request("shutdown", json!({})).await.unwrap();
        let status = tokio::time::timeout(Duration::from_secs(5), self.child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success());
    }
}

fn message(error: Value) -> String {
    error["message"].as_str().unwrap_or_default().to_owned()
}

#[tokio::test]
async fn protocol_info_lists_every_upstream_command() {
    let output = tokio::process::Command::new(BIN)
        .arg("--protocol-info")
        .output()
        .await
        .unwrap();
    assert!(output.status.success());
    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(info["id"], ID);
    assert_eq!(info["protocol"], 1);
    assert_eq!(
        info["commands"],
        json!([
            "paragraphs",
            "sentences",
            "words",
            "markdown",
            "html",
            "ai-generate"
        ])
    );
    let manifest: Value = serde_json::from_str(include_str!("../extension.json")).unwrap();
    let names: Vec<_> = manifest["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].clone())
        .collect();
    assert_eq!(json!(names), info["commands"]);
    assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest["schemaVersion"], 3);
    assert_eq!(manifest["protocolVersion"], 1);
    assert_eq!(manifest["replaces"][0], "raycast.lorem-ipsum");
}

#[tokio::test]
async fn structured_documents_use_each_output_preference_without_ai() {
    let mut p = Peer::start(ALL, ok()).await;
    for command in ["html", "markdown"] {
        for action in ["clipboard", "paste", "pasteAndCopy"] {
            let result = p
                .open("document", command, json!({"action":action}), json!({}))
                .await
                .unwrap();
            assert_eq!(result["mode"], "no-view");
            let calls = p.take();
            let methods: Vec<_> = calls.iter().map(|(method, _)| method.as_str()).collect();
            assert_eq!(
                methods,
                match action {
                    "clipboard" => vec!["host.clipboard.write", "host.window.visibility"],
                    "paste" => vec!["host.clipboard.pasteText"],
                    _ => vec!["host.clipboard.pasteText", "host.clipboard.write"],
                }
            );
            let text = calls[0].1["text"].as_str().unwrap();
            assert!((3..=8).contains(&text.split("\n\n").count()));
            assert!(if command == "html" {
                text.starts_with("<h2>") || text.starts_with("<h3>")
            } else {
                text.starts_with("## ") || text.starts_with("### ")
            });
            if action == "pasteAndCopy" {
                assert_eq!(calls[0].1, calls[1].1);
            }
        }
    }
    p.shutdown().await;
}

#[tokio::test]
async fn default_action_copies_then_hides() {
    let mut p = Peer::start(ALL, ok()).await;
    let result = p
        .open("a", "words", json!({}), json!({"numberOfWords":"5"}))
        .await
        .unwrap();
    assert_eq!(result["mode"], "no-view");
    assert_eq!(result["message"], "Copied to clipboard! 📋");
    assert_eq!(
        p.methods(),
        ["host.clipboard.write", "host.window.visibility"]
    );
    let text = p.calls[0].1["text"].as_str().unwrap();
    assert_eq!(text.split(' ').count(), 5);
    assert!(!text.ends_with('.'));
    assert_eq!(p.calls[1].1, json!({"visible":false}));
    p.shutdown().await;
}

#[tokio::test]
async fn every_action_preference_and_count_argument() {
    let mut p = Peer::start(ALL, ok()).await;
    let result = p
        .open(
            "p",
            "paragraphs",
            json!({"action":"paste"}),
            json!({"numberOfParagraphs":"3"}),
        )
        .await
        .unwrap();
    assert_eq!(result["message"], "Pasted to active app! 📝");
    let calls = p.take();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "host.clipboard.pasteText");
    let text = calls[0].1["text"].as_str().unwrap();
    assert_eq!(text.split("\r\n\r\n").count(), 3);

    let result = p
        .open(
            "s",
            "sentences",
            json!({"action":"pasteAndCopy"}),
            json!({"numberOfSentences":"4"}),
        )
        .await
        .unwrap();
    assert_eq!(
        result["message"],
        "Pasted to active app and copied to clipboard! 📋"
    );
    let calls = p.take();
    let methods: Vec<_> = calls.iter().map(|(m, _)| m.as_str()).collect();
    assert_eq!(
        methods,
        ["host.clipboard.pasteText", "host.clipboard.write"]
    );
    assert_eq!(calls[0].1, calls[1].1, "same text pasted and copied");
    assert_eq!(calls[0].1["text"].as_str().unwrap().matches('.').count(), 4);

    // Missing optional argument means one item.
    p.open("d", "paragraphs", json!({"action":"clipboard"}), json!({}))
        .await
        .unwrap();
    let calls = p.take();
    assert!(!calls[0].1["text"].as_str().unwrap().contains("\r\n"));
    p.shutdown().await;
}

#[tokio::test]
async fn invalid_input_never_touches_the_host() {
    let mut p = Peer::start(ALL, ok()).await;
    for (i, (command, argument, value)) in [
        ("paragraphs", "numberOfParagraphs", "1001"),
        ("sentences", "numberOfSentences", "lots"),
        ("words", "numberOfWords", "-2"),
    ]
    .into_iter()
    .enumerate()
    {
        let error = p
            .open(
                &format!("x{i}"),
                command,
                json!({}),
                json!({ argument: value }),
            )
            .await
            .unwrap_err();
        assert_eq!(message(error), INVALID);
    }
    let error = p
        .open("y", "words", json!({"action":"tweet"}), json!({}))
        .await
        .unwrap_err();
    assert!(message(error).contains("Default Action"));
    assert!(p.open("z", "nope", json!({}), json!({})).await.is_err());
    assert!(p.calls.is_empty(), "{:?}", p.calls);
    p.shutdown().await;
}

#[tokio::test]
async fn no_view_session_rejects_view_operations() {
    let mut p = Peer::start(ALL, ok()).await;
    p.request(
        "session.open",
        json!({"sessionId":"v","commandName":"words","preferences":{},"argumentValues":{}}),
    )
    .await
    .unwrap();
    p.take();
    for operation in ["extension:invoke-action", "extension:search-text-changed"] {
        assert!(p
            .request(
                "session.call",
                json!({"sessionId":"v","operation":operation,"payload":{"actionId":"x","searchText":"x"}}),
            )
            .await
            .is_err());
    }
    p.request("session.dispose", json!({"sessionId":"v"}))
        .await
        .unwrap();
    assert!(p.calls.is_empty());
    p.shutdown().await;
}

#[tokio::test]
async fn ai_generate_sends_original_prompt_and_outputs_trimmed_answer() {
    let mut p = Peer::start(
        ALL,
        Box::new(|method, _| match method {
            "host.ai.ask" => Ok(json!("  First paragraph.\n\nSecond paragraph.  ")),
            _ => Ok(Value::Null),
        }),
    )
    .await;
    let result = p
        .open(
            "ai",
            "ai-generate",
            json!({}),
            json!({"topic":"space cats"}),
        )
        .await
        .unwrap();
    assert_eq!(result["message"], "Copied to clipboard! 📋");
    assert_eq!(
        p.methods(),
        [
            "host.ai.ask",
            "host.clipboard.write",
            "host.window.visibility"
        ]
    );
    let ask = &p.calls[0].1;
    let prompt = ask["prompt"].as_str().unwrap();
    assert!(prompt.starts_with("\n    You are only capable of outputting a series of paragraphs."));
    assert!(prompt.contains("    - do not write in first person\n"));
    assert!(prompt.ends_with("The topic you should write about is \"space cats\".\n  "));
    assert_eq!(
        ask["options"],
        json!({"model":"OpenAI_GPT-4o_mini","creativity":"none"})
    );
    assert_eq!(
        p.calls[1].1["text"],
        "First paragraph.\n\nSecond paragraph."
    );
    p.take();
    // Empty topic picks a random topic and honours the paste preference.
    p.open(
        "ai2",
        "ai-generate",
        json!({"action":"paste"}),
        json!({"topic":""}),
    )
    .await
    .unwrap();
    assert!(p.calls[0].1["prompt"]
        .as_str()
        .unwrap()
        .contains("Pick a completely random topic to write about."));
    assert_eq!(p.calls[1].0, "host.clipboard.pasteText");
    p.shutdown().await;
}

#[tokio::test]
async fn ai_failures_are_reported_and_nothing_is_copied() {
    // No AI grant: the original "no access" failure, no reverse RPC at all.
    let mut p = Peer::start(&["clipboard.write", "window.visibility"], ok()).await;
    let error = p
        .open("a", "ai-generate", json!({}), json!({}))
        .await
        .unwrap_err();
    assert_eq!(message(error), "You don't have access to AI.");
    assert!(p.calls.is_empty());
    p.shutdown().await;

    // Provider error propagates unchanged.
    let mut p = Peer::start(
        ALL,
        Box::new(|method, _| match method {
            "host.ai.ask" => Err("No AI provider configured".into()),
            _ => Ok(Value::Null),
        }),
    )
    .await;
    let error = p
        .open("b", "ai-generate", json!({}), json!({}))
        .await
        .unwrap_err();
    assert!(message(error).contains("No AI provider configured"));
    assert_eq!(p.methods(), ["host.ai.ask"]);
    p.shutdown().await;

    // Blank answer is an error instead of an empty clipboard.
    let mut p = Peer::start(ALL, Box::new(|_, _| Ok(json!("   ")))).await;
    assert!(p
        .open("c", "ai-generate", json!({}), json!({}))
        .await
        .is_err());
    assert_eq!(p.methods(), ["host.ai.ask"]);
    p.shutdown().await;
}

#[tokio::test]
async fn clipboard_failures_never_report_success() {
    // Not granted: rejected locally by the SDK.
    let mut p = Peer::start(&[], ok()).await;
    assert!(p.open("a", "words", json!({}), json!({})).await.is_err());
    assert!(p
        .open("b", "words", json!({"action":"paste"}), json!({}))
        .await
        .is_err());
    assert!(p.calls.is_empty());
    p.shutdown().await;

    // Host write failure: error surfaces and Tezbar is not hidden.
    let mut p = Peer::start(
        ALL,
        Box::new(|method, _| match method {
            "host.clipboard.write" => Err("clipboard busy".into()),
            _ => Ok(Value::Null),
        }),
    )
    .await;
    let error = p
        .open("c", "sentences", json!({}), json!({}))
        .await
        .unwrap_err();
    assert!(message(error).contains("clipboard busy"));
    assert_eq!(p.methods(), ["host.clipboard.write"]);
    p.shutdown().await;

    // A failed hide after a successful copy is not a failed copy.
    let mut p = Peer::start(
        ALL,
        Box::new(|method, _| match method {
            "host.window.visibility" => Err("window gone".into()),
            _ => Ok(Value::Null),
        }),
    )
    .await;
    let result = p.open("d", "words", json!({}), json!({})).await.unwrap();
    assert_eq!(result["message"], "Copied to clipboard! 📋");
    p.shutdown().await;
}

#[tokio::test]
async fn zero_counts_and_whitespace_topic_keep_upstream_semantics() {
    let mut p = Peer::start(
        ALL,
        Box::new(|method, _| {
            if method == "host.ai.ask" {
                Ok(json!("Answer."))
            } else {
                Ok(Value::Null)
            }
        }),
    )
    .await;
    for (i, command, argument) in [
        (0, "words", "numberOfWords"),
        (1, "sentences", "numberOfSentences"),
        (2, "paragraphs", "numberOfParagraphs"),
    ] {
        p.open(
            &format!("zero{i}"),
            command,
            json!({}),
            json!({argument:"0"}),
        )
        .await
        .unwrap();
        let calls = p.take();
        let text = calls[0].1["text"].as_str().unwrap();
        match command {
            "words" => assert!((4..=16).contains(&text.split(' ').count())),
            "sentences" => assert!((4..=8).contains(&text.matches('.').count())),
            _ => assert_eq!(text, ""),
        }
    }
    p.open("topic", "ai-generate", json!({}), json!({"topic":"   "}))
        .await
        .unwrap();
    assert!(p.calls[0].1["prompt"]
        .as_str()
        .unwrap()
        .contains("The topic you should write about is \"   \"."));
    p.shutdown().await;
}

#[tokio::test]
async fn oversized_ai_output_is_rejected_before_clipboard_rpc() {
    let mut p = Peer::start(ALL, Box::new(|_, _| Ok(json!("x".repeat(1024 * 1024 + 1))))).await;
    let error = p
        .open("large", "ai-generate", json!({}), json!({}))
        .await
        .unwrap_err();
    assert!(message(error).contains("at most 1048576 bytes"));
    assert_eq!(p.methods(), ["host.ai.ask"]);
    p.shutdown().await;
}
