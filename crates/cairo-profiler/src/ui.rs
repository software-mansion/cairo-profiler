//! UI utilities for the Cairo profiler tool.
//! All human-oriented messaging must use this module to communicate with the user.
use console::style;
use std::fmt::Display;

/// Print an warning message.
pub fn warn(message: impl Display) {
    let tag = style("WARNING").yellow();
    eprintln!("[{tag}] {message}");
}

/// Print a message.
pub fn msg(message: impl Display) {
    println!("{message}");
}
