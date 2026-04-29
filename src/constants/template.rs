use crate::constants::{OUTPUT_SYMBOL_ERROR, OUTPUT_SYMBOL_WARNING};

pub fn render_template(template: &str, values: &[String]) -> String {
    let mut rendered = template.to_string();
    for value in values {
        rendered = rendered.replacen("{}", value, 1);
    }
    rendered
}

pub fn render_template_from_iter<I, T>(template: &str, values: I) -> String
where
    I: IntoIterator<Item = T>,
    T: ToString,
{
    let collected: Vec<String> = values.into_iter().map(|value| value.to_string()).collect();
    render_template(template, &collected)
}

pub fn render_with_warning(template: &str, other_args: &[String]) -> String {
    use colored::Colorize;
    let mut args = vec![OUTPUT_SYMBOL_WARNING.yellow().to_string()];
    args.extend_from_slice(other_args);
    render_template(template, &args)
}

pub fn render_with_error(template: &str, other_args: &[String]) -> String {
    use colored::Colorize;
    let mut args = vec![OUTPUT_SYMBOL_ERROR.red().bold().to_string()];
    args.extend_from_slice(other_args);
    render_template(template, &args)
}
