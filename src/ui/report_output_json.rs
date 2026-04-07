use crate::constants::{OUTPUT_JSON_SERIALIZATION_ERROR_TEMPLATE, render_template};
use crate::types::Report;

pub(super) fn print_json(report: &Report) {
    match serde_json::to_string_pretty(report) {
        Ok(json) => println!("{json}"),
        Err(error) => eprintln!(
            "{}",
            render_template(
                OUTPUT_JSON_SERIALIZATION_ERROR_TEMPLATE,
                &[error.to_string()]
            )
        ),
    }
}
