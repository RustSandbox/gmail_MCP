use crate::EmailSummary;
use crate::url_remover::UrlRemover;
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

    // After HTML conversion (or if it was already plain text), remove URLs
    tracing::debug!("Removing URLs from email body");
    summary.body_raw = remove_urls_from_text(&summary.body_raw);
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

/// Function to remove URLs from email body text
///
/// This function creates a UrlRemover instance and uses it to clean URLs
/// from the email body content, providing cleaner text for analysis.
///
/// # Arguments
/// * `text` - The email body text that may contain URLs
///
/// # Returns
/// * `String` - The cleaned text with URLs removed
///
/// # Example
/// ```
/// let cleaned = remove_urls_from_text("Check this out: https://example.com");
/// // Result: "Check this out: "
/// ```
pub fn remove_urls_from_text(text: &str) -> String {
    match UrlRemover::new() {
        Ok(remover) => {
            tracing::debug!("Removing URLs from text of length {}", text.len());
            let cleaned = remover.clean_text(text);
            tracing::debug!(
                "Text cleaned: {} -> {} characters",
                text.len(),
                cleaned.len()
            );
            cleaned
        }
        Err(e) => {
            tracing::warn!(
                "Failed to create URL remover: {:?}, returning original text",
                e
            );
            text.to_string()
        }
    }
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

    #[test]
    fn test_url_removal() {
        // Test URL removal functionality
        let test_text = "Check out this link: https://example.com and this one too: www.test.org. More text here.";
        let cleaned = remove_urls_from_text(test_text);

        println!("Original: {}", test_text);
        println!("Cleaned: {}", cleaned);

        // Verify URLs are removed
        assert!(!cleaned.contains("https://example.com"));
        assert!(!cleaned.contains("www.test.org"));
        assert!(cleaned.contains("Check out this link:"));
        assert!(cleaned.contains("More text here."));
    }

    #[tokio::test]
    async fn test_email_processing_with_urls() {
        // Test email processing that includes URL removal
        let mut email = crate::EmailSummary {
            id: "test_id".to_string(),
            from: "test@example.com".to_string(),
            subject: "Test Subject".to_string(),
            snippet: "Test snippet".to_string(),
            body_raw: "Check this out: https://example.com\n\nVisit www.test.org for more info.\n\nThanks!".to_string(),
        };

        // Process the email (this should remove URLs)
        convert_html_to_text(&mut email).await;

        // Verify URLs were removed
        assert!(!email.body_raw.contains("https://example.com"));
        assert!(!email.body_raw.contains("www.test.org"));
        assert!(email.body_raw.contains("Check this out:"));
        assert!(email.body_raw.contains("Thanks!"));

        println!("Processed email body: {}", email.body_raw);
    }
}
