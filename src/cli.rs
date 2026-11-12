//! Reaching the command line from a menu that cannot type.
//!
//! Configuring the gateway — providers, accounts, routes, client keys — is
//! text: a channel id, a base URL, an API key. A menu cannot ask for those, and
//! an `NSAlert` per field is four dialogs to add one provider. So the menu
//! hands the job to the interface that does it well, and opens a terminal on
//! the copy of `xproxy` that ships inside this bundle.

use std::path::PathBuf;

/// The `xproxy` binary that belongs to this installation.
///
/// Sibling of the running executable, which is `Contents/MacOS/opus` inside a
/// bundle. Falls back to `xproxy` on `PATH` for a `cargo run` from source.
pub fn xproxy_binary() -> PathBuf {
    if let Ok(current) = std::env::current_exe()
        && let Some(dir) = current.parent()
    {
        let sibling = dir.join("xproxy");
        if sibling.is_file() {
            return sibling;
        }
    }
    PathBuf::from("xproxy")
}

/// Open Terminal on the CLI, showing where things stand.
pub fn open_terminal() {
    let binary = xproxy_binary();
    let line = format!("'{}' status", binary.display());
    let script = format!(
        "tell application \"Terminal\"\ndo script \"{}\"\nactivate\nend tell",
        line.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let _ = std::process::Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .status();
}
