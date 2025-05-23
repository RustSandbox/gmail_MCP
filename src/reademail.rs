use crate::EmailSummary;
use html2text::from_read as html_to_text;
use tokio::task;
use tracing_subscriber;

/// Reads emails from Gmail, processes them, and outputs as JSON.
///
/// This function fetches emails via the Gmail API, converts any HTML content to plain text,
/// and outputs the processed emails as formatted JSON.
///
/// # Arguments
/// * `max_results` - The maximum number of emails to fetch (1-500, as per Gmail API limits)
pub async fn read_emails(max_results: u32) -> Result<String, Box<dyn std::error::Error>> {
    // Set up tracing
    initialize_logging()?;
    tracing::info!(max_results, "Starting gmailrs application");

    // Fetch emails from Gmail API
    let json = crate::run(max_results).await?;
    let mut response: crate::EmailResponse = serde_json::from_str(&json)?;

    // Process emails (convert HTML to text)
    response.emails = process_email_summaries(response.emails).await;

    // Return the complete response as JSON
    let result_json = serde_json::to_string_pretty(&response)?;
    Ok(result_json)
}

/// Initialize the logging infrastructure
fn initialize_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Use try_init() instead of init() to avoid panic if already initialized
    // This allows the function to work both in standalone mode and when called from the server
    match tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Default to INFO level
        .try_init()
    {
        Ok(_) => {
            // Successfully initialized
            Ok(())
        }
        Err(_) => {
            // Already initialized, which is fine when running as part of the server
            Ok(())
        }
    }
}

/// Process email summaries and output as JSON
async fn process_and_output_emails(
    summaries: Vec<EmailSummary>,
) -> Result<String, Box<dyn std::error::Error>> {
    tracing::info!("Processing {} email summaries", summaries.len());

    // Process emails
    let converted = process_email_summaries(summaries).await;

    // Output as JSON
    tracing::info!("All messages processed, returning JSON");
    let json_output = serde_json::to_string_pretty(&converted)?;

    Ok(json_output)
}

pub async fn convert_html_to_text(summary: &mut EmailSummary) {
    if summary.body_raw.starts_with("<") {
        // Spawn a blocking task to perform the HTML to text conversion
        let html_body = summary.body_raw.clone();
        let plain_text = task::spawn_blocking(move || html_to_text(html_body.as_bytes(), 100))
            .await
            .unwrap();

        summary.body_raw = plain_text;
    } else {
        tracing::debug!("Body is not HTML, skipping conversion");
    }
}

/// Process email summaries by converting HTML content to plain text.
async fn process_email_summaries(summaries: Vec<EmailSummary>) -> Vec<EmailSummary> {
    let mut converted: Vec<EmailSummary> = Vec::with_capacity(summaries.len());

    for (idx, mut summary) in summaries.into_iter().enumerate() {
        tracing::debug!(msg_index = idx, id = %summary.id, "Converting body if HTML");
        convert_html_to_text(&mut summary).await;
        converted.push(summary);
        tracing::debug!(msg_index = idx, "Message processing done");

        // Introduce a small delay to avoid overwhelming the system
        //time::sleep(Duration::from_millis(50)).await;
    }

    converted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    pub async fn async_read_emails() {
        // Test with fetching 10 emails
        let result = read_emails(10).await.unwrap();

        // Parse and display the response
        if let Ok(response) = serde_json::from_str::<crate::EmailResponse>(&result) {
            println!("----------------------------------");
            println!("Fetched {} emails:", response.count);
            for email in response.emails {
                println!("From: {}", email.from);
                println!("Subject: {}", email.subject);
                println!("---");
            }
            println!("----------------------------------");
        }
    }
}
