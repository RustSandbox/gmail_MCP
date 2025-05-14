//! Minimal binary that delegates all Gmail logic to the library.

/// This binary exists solely to bootstrap a Tokio runtime and call the
/// `gmailrs::run` function provided by the library (see `src/lib.rs`).
/// All Gmail-related logic, authentication, and API calls live in the library.
use gmailrs::EmailSummary;
use html2text::from_read as html_to_text;
use std::time::Duration;
use tokio::{task, time};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting gmailrs application");

    let json = gmailrs::run().await?;
    let summaries: Vec<EmailSummary> = serde_json::from_str(&json)?;

    // Convert bodies that look like HTML to Markdown.
    //
    // We run the conversion inside a blocking task with a **time-out** so that
    // malformed or extremely large HTML fragments do **not** freeze the entire
    // application (see known issues in html2md/html5ever regarding infinite
    // loops for certain inputs).
    //
    // If the conversion exceeds 500 ms we fall back to the raw body and emit a
    // warning via `tracing`.
    let mut converted: Vec<EmailSummary> = Vec::with_capacity(summaries.len());

    for (idx, mut s) in summaries.into_iter().enumerate() {
        tracing::debug!(msg_index = idx, id = %s.id, "Converting body if HTML");
        // Only attempt conversion if the body *looks* like HTML.
        if s.body_raw.trim_start().starts_with('<') {
            let html = s.body_raw.clone();

            // Spawn the CPU-heavy conversion (HTML → plain text) on a blocking thread.
            let handle = task::spawn_blocking(move || html_to_text(html.as_bytes(), 80));

            match time::timeout(Duration::from_millis(500), handle).await {
                // Conversion completed in time → use the plain-text version.
                Ok(Ok(txt)) => {
                    tracing::debug!(msg_index = idx, "HTML→text conversion succeeded");
                    s.body_raw = txt;
                }

                // Conversion panicked or the task failed ✗ → keep raw.
                Ok(Err(e)) => {
                    tracing::warn!(error = ?e, "html→text conversion failed – keeping raw HTML");
                }

                // We hit the 500 ms deadline ✗ → keep raw and move on.
                Err(_) => {
                    tracing::warn!("html→text conversion timed out – keeping raw HTML");
                }
            }
        }

        converted.push(s);

        tracing::debug!(msg_index = idx, "Message processing done");
    }

    tracing::info!("All messages processed, outputting JSON");
    println!("{}", serde_json::to_string_pretty(&converted)?);

    Ok(())
}
