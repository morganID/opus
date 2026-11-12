//! The clipboard, and nothing else.

use std::io::Write;

/// `pbcopy` rather than a crate: one process, always present.
pub fn copy(text: &str) -> std::io::Result<()> {
    let mut child = std::process::Command::new("/usr/bin/pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    child.wait()?;
    Ok(())
}
