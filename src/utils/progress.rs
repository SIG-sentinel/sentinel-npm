use crate::constants::{PROGRESS_TICK_CHARS, PROGRESS_TICK_MS};
use crate::types::{OutputFormat, ProgressBarConfig};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::time::Duration;

pub fn create_progress_bar(config: ProgressBarConfig) -> ProgressBar {
    let progress_bar = ProgressBar::new(config.length as u64);
    let style = ProgressStyle::with_template(config.template)
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .tick_chars(PROGRESS_TICK_CHARS);

    progress_bar.set_style(style);
    progress_bar.set_message(config.message);
    progress_bar.enable_steady_tick(Duration::from_millis(PROGRESS_TICK_MS));
    progress_bar
}

pub fn should_render_progress_bar(output_format: &OutputFormat, quiet: bool) -> bool {
    !quiet && matches!(output_format, OutputFormat::Text) && std::io::stderr().is_terminal()
}
