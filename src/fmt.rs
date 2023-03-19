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

pub(crate) fn plain_headline(msg: impl Display) -> String {
    format!("❯ {}", console::style(msg).bold())
}
