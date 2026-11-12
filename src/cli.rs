//! Reaching the command line from a menu that cannot type.
//!
//! Configuring the gateway — providers, accounts, routes, client keys — is
//! text: a channel id, a base URL, an API key. A menu cannot ask for those, and
//! an `NSAlert` per field is four dialogs to add one provider. So the menu
//! hands the job to the interface that does it well, and opens a terminal on
//! the copy of `xproxy` that ships inside this bundle.

use std::path::PathBuf;

/// The `xproxy` binary, wherever it was installed.
///
/// Opus does not carry it: the gateway is its own product with its own release
/// and its own installer, and a menu bar app that quietly shipped a second
/// copy would be a second copy to keep in step. `OPUS_XPROXY` overrides the
/// search, which is what a development build uses.
pub fn xproxy_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("OPUS_XPROXY") {
        return Some(PathBuf::from(path));
    }
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/xproxy"),
        PathBuf::from("/usr/local/bin/xproxy"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local/bin/xproxy"));
        candidates.push(home.join(".cargo/bin/xproxy"));
    }
    candidates.into_iter().find(|path| path.is_file())
}

/// Open Terminal on the CLI, showing where things stand.
///
/// With no `xproxy` installed there is nothing to open, so the window says so
/// and gives the line that fixes it rather than failing silently.
pub fn open_terminal() {
    let line = match xproxy_binary() {
        Some(binary) => format!("'{}' status", binary.display()),
        None => concat!(
            "echo 'xproxy is not installed. Install it with:';",
            "echo;",
            "echo '  curl -fsSL https://raw.githubusercontent.com/morganID/xproxy/main/scripts/install.sh | sh';",
            "echo"
        )
        .to_owned(),
    };
    let script = format!(
        "tell application \"Terminal\"\ndo script \"{}\"\nactivate\nend tell",
        line.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let _ = std::process::Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .status();
}
