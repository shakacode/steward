use std::fmt::Display;

/// Formats a headline that gets printed to console when running a command.
///
/// ```sh
/// ❯ Building server: $ cargo build [@ my-project/server]
/// ```
#[macro_export]
macro_rules! headline {
    ($cmd:expr) => {{
        let cmd = console::style(format!("$ {} [@ {}]", $cmd.exe(), $cmd.pwd().display())).dim();
        match $cmd.msg() {
            Some(msg) => format!("❯ {} {}", console::style(format!("{}:", msg)).bold(), cmd),
            None => format!("❯ {}", cmd),
        }
    }};
}

/// Prints a formatted message to console.
///
/// ```rust
/// steward::print("Nothing to do. Exiting.")
/// ```
///
/// would print
///
/// ```sh
/// ❯ Nothing to do. Exiting.
/// ```
pub fn print(msg: impl Display) {
    eprintln!("{}", self::plain_headline(msg))
}

pub(crate) fn plain_headline(msg: impl Display) -> String {
    format!("❯ {}", console::style(msg).bold())
}
