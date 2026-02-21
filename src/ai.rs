//! AI-powered commit message generation using `claude -p`.

use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// System prompt for commit message generation.
const COMMIT_MSG_SYSTEM_PROMPT: &str = r#"You are a commit message generator. Given a git diff, generate a concise commit message following Conventional Commits format: `type: summary`

Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore

Rules:
- First line: type: short summary (max 72 chars)
- Use imperative mood ("add" not "added")
- No period at the end
- Be specific about what changed
- Output ONLY the commit message, nothing else"#;

/// Generate a commit message from a diff using `claude -p`.
///
/// Returns the generated message or an error if `claude` is not available.
pub fn generate_commit_message(diff_text: &str, model: &str) -> Result<String> {
    let mut child = Command::new("claude")
        .args(["-p", "--output-format", "text", "--max-turns", "1"])
        .env("ANTHROPIC_MODEL", model)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to run 'claude' command. Is Claude Code CLI installed?")?;

    // Write the prompt + diff to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let prompt = format!(
            "{}\n\nGenerate a commit message for the following diff:\n\n{}",
            COMMIT_MSG_SYSTEM_PROMPT, diff_text
        );
        stdin
            .write_all(prompt.as_bytes())
            .context("Failed to write to claude stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for claude process")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("claude command failed: {stderr}");
    }

    let message = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if message.is_empty() {
        anyhow::bail!("claude returned empty response");
    }

    Ok(message)
}

/// Check if the `claude` CLI is available.
#[allow(dead_code)]
pub fn is_claude_available() -> bool {
    Command::new("claude")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}
