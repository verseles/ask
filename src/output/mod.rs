//! Output formatting module

mod colorize;
mod formatter;
mod markdown;
mod spinner;

pub use formatter::*;
pub use spinner::{Spinner, StreamingIndicator};
