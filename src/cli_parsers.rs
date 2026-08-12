use crate::constants::{
    CLI_PARSER_ERR_DURATION_TOO_LARGE, CLI_PARSER_ERR_EMPTY_TIMESTAMP,
    CLI_PARSER_ERR_EXPECTED_POSITIVE_INTEGER, CLI_PARSER_ERR_INVALID_TIMESTAMP,
    CLI_PARSER_KEYWORD_NOW, CLI_PARSER_SUFFIX_AGO,
};

fn parse_absolute_rfc3339_timestamp(trimmed: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(trimmed)
        .ok()
        .map(|timestamp| timestamp.to_rfc3339())
}

fn parse_now_keyword_timestamp(normalized: &str) -> Option<String> {
    (normalized == CLI_PARSER_KEYWORD_NOW).then(|| chrono::Utc::now().to_rfc3339())
}

fn parse_relative_timestamp(trimmed: &str, normalized: &str) -> Result<String, String> {
    let relative_duration_text = normalized
        .strip_suffix(CLI_PARSER_SUFFIX_AGO)
        .map_or(trimmed, str::trim);

    let duration = humantime::parse_duration(relative_duration_text)
        .map_err(|_| CLI_PARSER_ERR_INVALID_TIMESTAMP.to_string())?;

    let chrono_duration = chrono::Duration::from_std(duration)
        .map_err(|_| CLI_PARSER_ERR_DURATION_TOO_LARGE.to_string())?;

    let relative_timestamp = (chrono::Utc::now() - chrono_duration).to_rfc3339();

    Ok(relative_timestamp)
}

pub(crate) fn parse_rfc3339_timestamp(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let is_empty_timestamp = trimmed.is_empty();

    if is_empty_timestamp {
        return Err(CLI_PARSER_ERR_EMPTY_TIMESTAMP.to_string());
    }

    let normalized = trimmed.to_ascii_lowercase();

    parse_absolute_rfc3339_timestamp(trimmed)
        .or_else(|| parse_now_keyword_timestamp(&normalized))
        .map_or_else(|| parse_relative_timestamp(trimmed, &normalized), Ok)
}

pub(crate) fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| CLI_PARSER_ERR_EXPECTED_POSITIVE_INTEGER.to_string())?;

    let is_positive = parsed > 0;

    if !is_positive {
        return Err(CLI_PARSER_ERR_EXPECTED_POSITIVE_INTEGER.to_string());
    }

    Ok(parsed)
}
