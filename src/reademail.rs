use crate::{EmailResponse, EmailSummary};
use html2text::from_read as html_to_text;
use regex::Regex;
use tokio::task;
use tracing::{error, info, warn};

/// Reads emails from Gmail and processes them
pub async fn read_emails(max_results: u32) -> Result<String, Box<dyn std::error::Error>> {
    info!("Starting to read {} emails from Gmail", max_results);

    let json = match crate::run(max_results).await {
        Ok(json) => {
            info!("Gmail API response received ({} bytes)", json.len());
            json
        }
        Err(e) => {
            error!("Failed to fetch emails from Gmail API: {}", e);
            return Err(e);
        }
    };

    let mut response: EmailResponse = serde_json::from_str(&json)?;

    if response.emails.is_empty() {
        warn!("No emails found in Gmail response");
        return Ok(serde_json::to_string_pretty(&response)?);
    }

    info!("Processing {} emails", response.emails.len());
    for email in response.emails.iter_mut() {
        convert_html_to_text(email).await;
    }

    info!("Email processing completed");
    Ok(serde_json::to_string_pretty(&response)?)
}

/// Convert HTML to text and remove URLs
pub async fn convert_html_to_text(summary: &mut EmailSummary) {
    // Convert HTML to text if needed
    if summary.body_raw.starts_with('<') {
        let html_body = summary.body_raw.clone();
        let plain_text = task::spawn_blocking(move || html_to_text(html_body.as_bytes(), 100))
            .await
            .unwrap();
        summary.body_raw = plain_text;
    }

    // Remove URLs from text (simplified inline implementation)
    summary.body_raw = remove_urls_simple(&summary.body_raw);
}

/// Simple URL removal function
fn remove_urls_simple(text: &str) -> String {
    // Simple regex to match common URL patterns
    let url_regex = Regex::new(r"https?://[^\s]+|www\.[^\s]+").unwrap_or_else(|_| {
        // If regex fails, return a regex that matches nothing
        Regex::new(r"$^").unwrap()
    });

    url_regex.replace_all(text, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_removal() {
        let test_text = "Check out this link: https://example.com and this one too: www.test.org. More text here.";
        let cleaned = remove_urls_simple(test_text);

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
