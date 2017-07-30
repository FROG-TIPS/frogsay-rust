/// Utilities for formatting the spouting frog.

use itertools::Itertools;
use textwrap::Wrapper;

/// Return a string with a frog spouting the given `text` in a speech bubble. The speech
/// text will be wrapped to the current terminal width. Distinct paragraphs are supported and
/// can be specified by inserting newlines in `text`. Whitespace immediately after or before
/// each newline will be trimmed.
///
/// # Arguments
/// - `text` - Newline-separated text to display.
pub fn say<S>(text: S) -> String
where
    S: Into<String>,
{
    let indent = "        ";
    let wrapper = Wrapper::with_termwidth()
        .subsequent_indent(indent)
        .initial_indent(indent)
        .squeeze_whitespace(false)
        .break_words(true);

    let wrapped = text.into()
        .lines()
        .map(|p| wrapper.fill(&format!("{}", p)))
        .join("\n");

    format!(
        r#"
{text}
{indent}/
  @..@
 (----)
( >__< )
^^ ~~ ^^"#,
        indent = indent,
        text = wrapped
    )
}
