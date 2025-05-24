use crate::url_remover::UrlRemover;
use crate::{EmailResponse, EmailSummary};
use html2text::from_read as html_to_text;
use tokio::task;
use tracing::{error, info, warn};

/// Reads emails from Gmail and processes them
pub async fn read_emails(max_results: u32) -> Result<String, Box<dyn std::error::Error>> {
    info!("Starting to read {} emails from Gmail", max_results);

    let json = match crate::run(max_results).await {
        Ok(json) => {
            info!(
                "Raw Gmail API response received, length: {} bytes",
                json.len()
            );
            json
        }
        Err(e) => {
            error!("Failed to fetch emails from Gmail API: {}", e);
            return Err(e);
        }
    };

    let mut response: EmailResponse = match serde_json::from_str::<EmailResponse>(&json) {
        Ok(response) => {
            info!("Parsed {} emails from Gmail response", response.count);
            response
        }
        Err(e) => {
            error!("Failed to parse Gmail response JSON: {}", e);
            return Err(e.into());
        }
    };

    if response.emails.is_empty() {
        warn!("No emails found in Gmail response");
        return Ok(serde_json::to_string_pretty(&response)?);
    }

    // Process emails (convert HTML to text and remove URLs)
    let email_count = response.emails.len();
    info!(
        "Processing {} emails (HTML to text conversion and URL removal)",
        email_count
    );
    for (i, email) in response.emails.iter_mut().enumerate() {
        info!(
            "Processing email {}/{}: {}",
            i + 1,
            email_count,
            email.subject
        );
        convert_html_to_text(email).await;
    }

    info!("Email processing completed successfully");
    Ok(serde_json::to_string_pretty(&response)?)
}

/// Convert HTML to text and remove URLs
pub async fn convert_html_to_text(summary: &mut EmailSummary) {
    // Convert HTML to text if needed
    if summary.body_raw.starts_with("<") {
        let html_body = summary.body_raw.clone();
        let plain_text = task::spawn_blocking(move || html_to_text(html_body.as_bytes(), 100))
            .await
            .unwrap();
        summary.body_raw = plain_text;
    }

    // Remove URLs from text
    summary.body_raw = remove_urls_from_text(&summary.body_raw);
}

/// Remove URLs from email body text
pub fn remove_urls_from_text(text: &str) -> String {
    match UrlRemover::new() {
        Ok(remover) => remover.clean_text(text),
        Err(_) => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_removal() {
        let test_text = "Check out this link: https://example.com and this one too: www.test.org. More text here.";
        let cleaned = remove_urls_from_text(test_text);

        assert!(!cleaned.contains("https://example.com"));
        assert!(!cleaned.contains("www.test.org"));
        assert!(cleaned.contains("Check out this link:"));
        assert!(cleaned.contains("More text here."));
    }

    #[tokio::test]
    async fn test_email_processing_with_urls() {
        let mut email = EmailSummary {
            id: "test_id".to_string(),
            from: "test@example.com".to_string(),
            subject: "Test Subject".to_string(),
            snippet: "Test snippet".to_string(),
            body_raw: "Check this out: https://example.com\n\nVisit www.test.org for more info.\n\nThanks!".to_string(),
        };

        convert_html_to_text(&mut email).await;

        assert!(!email.body_raw.contains("https://example.com"));
        assert!(!email.body_raw.contains("www.test.org"));
        assert!(email.body_raw.contains("Check this out:"));
        assert!(email.body_raw.contains("Thanks!"));
    }
}
