//! Minimal binary that delegates all Gmail logic to the library.

/// This binary exists solely to bootstrap a Tokio runtime and call the
/// `gmailrs::run` function provided by the library (see `src/lib.rs`).
/// All Gmail-related logic, authentication, and API calls live in the library.
use gmailrs::EmailSummary;
use html2text::from_read as html_to_text;
use std::time::Duration;
use tokio::{task, time};
use tracing_subscriber::fmt;

// Import the parser module for infix to postfix conversion.
mod parser;
mod utils;
use parser::{ParseError, infix_to_postfix};
use utils::{convert_html_to_text, demo_infix_to_postfix};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting gmailrs application");

    let json = gmailrs::run().await?;
    let summaries: Vec<EmailSummary> = serde_json::from_str(&json)?;

    let mut converted: Vec<EmailSummary> = Vec::with_capacity(summaries.len());

    for (idx, mut s) in summaries.into_iter().enumerate() {
        tracing::debug!(msg_index = idx, id = %s.id, "Converting body if HTML");
        convert_html_to_text(&mut s).await;
        converted.push(s);
        tracing::debug!(msg_index = idx, "Message processing done");
    }

    tracing::info!("All messages processed, outputting JSON");
    println!("{}", serde_json::to_string_pretty(&converted)?);

    // Call the infix-to-postfix demo function
    demo_infix_to_postfix();

    Ok(())
}
