use crate::parser::{ParseError, infix_to_postfix};
use gmailrs::EmailSummary;
use html2text::from_read as html_to_text;
use std::time::Duration;
use tokio::{task, time};

/// Demonstrates converting an infix expression to postfix notation using the parser module.
/// This function prints the result or error to the console.
pub fn demo_infix_to_postfix() {
    // Example infix expression (only single-digit numbers and '+'/'-' are supported)
    let infix = "2+3-4";
    match infix_to_postfix(infix) {
        Ok(postfix) => {
            println!("Infix: {infix} => Postfix: {postfix}");
        }
        Err(e) => {
            println!("Failed to parse infix expression: {e:?}");
        }
    }
}

/// Converts the body of an EmailSummary from HTML to plain text if it looks like HTML.
/// This function uses a blocking task with a timeout to prevent freezing the application.
///
/// Arguments:
///     summary: &mut EmailSummary - The email summary whose body is to be converted.
///
/// Returns:
///     () - The summary is updated in place.
pub async fn convert_html_to_text(summary: &mut EmailSummary) {
    if summary.body_raw.trim_start().starts_with('<') {
        let html = summary.body_raw.clone();
        let handle = task::spawn_blocking(move || html_to_text(html.as_bytes(), 80));
        match time::timeout(Duration::from_millis(500), handle).await {
            Ok(Ok(txt)) => {
                tracing::debug!("HTML→text conversion succeeded");
                summary.body_raw = txt;
            }
            Ok(Err(e)) => {
                tracing::warn!(error = ?e, "html→text conversion failed – keeping raw HTML");
            }
            Err(_) => {
                tracing::warn!("html→text conversion timed out – keeping raw HTML");
            }
        }
    }
}
