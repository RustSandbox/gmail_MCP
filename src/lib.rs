//! Gmail API Client Library
//! Simple library for fetching Gmail emails with OAuth2 authentication.

pub mod reademail;
pub mod url_remover;

use gmail1::hyper_rustls::HttpsConnectorBuilder;
use gmail1::hyper_util::{client::legacy::Client, rt::TokioExecutor};
use gmail1::{
    api::{ListMessagesResponse, MessagePart},
    Gmail,
};
use google_gmail1 as gmail1;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

/// Lightweight representation of an email message that our API returns.
#[derive(Serialize, Deserialize, Debug)]
pub struct EmailSummary {
    /// The unique Gmail message ID.
    pub id: String,
    /// The value of the `From` header.
    pub from: String,
    /// The value of the `Subject` header.
    pub subject: String,
    /// A short snippet of the message body.
    pub snippet: String,
    /// Raw body (HTML or plain text).
    pub body_raw: String,
}

/// Response structure that wraps the email summaries
#[derive(Serialize, Deserialize, Debug)]
pub struct EmailResponse {
    /// The list of email summaries
    pub emails: Vec<EmailSummary>,
    /// The total number of emails fetched
    pub count: usize,
}

/// Extract the plain-text body from a `Message`. Falls back to empty string.
fn bytes_to_string(data: &[u8]) -> Option<String> {
    String::from_utf8(data.to_vec()).ok()
}

fn extract_body(msg: &gmail1::api::Message) -> String {
    if let Some(payload) = &msg.payload {
        // Try top-level body first
        if let Some(body) = &payload.body {
            if let Some(data) = &body.data {
                if let Some(txt) = bytes_to_string(data) {
                    return txt;
                }
            }
        }

        // Search parts for text/plain
        if let Some(parts) = &payload.parts {
            if let Some(txt) = find_plain_text(parts) {
                return txt;
            }
        }
    }
    String::new()
}

/// Recursively traverse message parts to find the first `text/plain` body.
fn find_plain_text(parts: &[MessagePart]) -> Option<String> {
    for part in parts {
        if part.mime_type.as_deref() == Some("text/plain") {
            if let Some(body) = &part.body {
                if let Some(data) = &body.data {
                    if let Some(txt) = bytes_to_string(data) {
                        return Some(txt);
                    }
                }
            }
        }

        // Recurse into sub-parts
        if let Some(sub_parts) = &part.parts {
            if let Some(txt) = find_plain_text(sub_parts) {
                return Some(txt);
            }
        }
    }
    None
}

/// Handle authentication errors with helpful instructions
fn handle_auth_error() -> String {
    error!("Gmail API: Authentication failed - token may be expired or invalid");
    warn!("Gmail API: To fix authentication issues:");
    warn!("Gmail API: 1. Delete 'token_cache.json' file");
    warn!("Gmail API: 2. Restart the server");
    warn!("Gmail API: 3. Re-authenticate when browser opens");
    warn!("Gmail API: 4. Make sure Gmail API is enabled in Google Cloud Console");
    "Authentication failed. Please delete token_cache.json and restart the server to re-authenticate.".to_string()
}

/// Fetch Gmail emails using OAuth2 authentication
pub async fn run(max_results: u32) -> Result<String, Box<dyn std::error::Error>> {
    let max_results = max_results.min(500).max(1);
    info!("Gmail API: Starting to fetch {} emails", max_results);

    // Load credentials
    info!("Gmail API: Loading credentials from client_secret.json");
    let secret = yup_oauth2::read_application_secret("client_secret.json")
        .await
        .map_err(|e| {
            error!("Gmail API: Failed to read client_secret.json: {}", e);
            e
        })?;

    // Set up authenticator
    info!("Gmail API: Setting up OAuth2 authenticator");
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("token_cache.json")
        .build()
        .await
        .map_err(|e| {
            error!("Gmail API: Failed to build authenticator: {}", e);
            e
        })?;

    // Create HTTPS client
    info!("Gmail API: Creating HTTPS client");
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http1()
        .build();

    let client = Client::builder(TokioExecutor::new()).build(https);
    let hub = Gmail::new(client, auth);

    // Fetch messages
    info!("Gmail API: Requesting message list from inbox");
    let result = hub
        .users()
        .messages_list("me")
        .q("in:inbox")
        .max_results(max_results)
        .doit()
        .await
        .map_err(|e| {
            error!("Gmail API: Failed to list messages: {}", e);
            e
        })?;

    let mut summaries = Vec::new();

    if let (
        _,
        ListMessagesResponse {
            messages: Some(messages),
            ..
        },
    ) = result
    {
        let message_count = messages.len();
        info!(
            "Gmail API: Found {} messages, fetching details",
            message_count
        );

        for (i, message) in messages.into_iter().enumerate() {
            if let Some(id) = message.id {
                info!(
                    "Gmail API: Fetching message {}/{}: {}",
                    i + 1,
                    message_count,
                    id
                );

                match hub
                    .users()
                    .messages_get("me", &id)
                    .format("full")
                    .add_scope("https://www.googleapis.com/auth/gmail.readonly")
                    .doit()
                    .await
                {
                    Ok((_, msg)) => {
                        if let Some(payload) = &msg.payload {
                            if let Some(headers) = &payload.headers {
                                let subject = headers
                                    .iter()
                                    .find(|h| h.name.as_deref() == Some("Subject"))
                                    .and_then(|h| h.value.clone())
                                    .unwrap_or_else(|| "No Subject".to_string());

                                let from = headers
                                    .iter()
                                    .find(|h| h.name.as_deref() == Some("From"))
                                    .and_then(|h| h.value.clone())
                                    .unwrap_or_else(|| "Unknown Sender".to_string());

                                let snippet = msg.snippet.clone().unwrap_or_default();
                                let body_raw = extract_body(&msg);

                                summaries.push(EmailSummary {
                                    id: id.clone(),
                                    from,
                                    subject: subject.clone(),
                                    snippet,
                                    body_raw,
                                });

                                info!("Gmail API: Successfully processed email: {}", subject);
                            } else {
                                warn!("Gmail API: Message {} has no headers", id);
                            }
                        } else {
                            warn!("Gmail API: Message {} has no payload", id);
                        }
                    }
                    Err(e) => {
                        error!("Gmail API: Failed to fetch message {}: {}", id, e);
                        // Check if it's an authentication error
                        if e.to_string().contains("403")
                            || e.to_string().contains("PERMISSION_DENIED")
                        {
                            error!("Gmail API: This appears to be an authentication issue");
                            warn!("Gmail API: Consider deleting token_cache.json and restarting");
                        }
                    }
                }
            } else {
                warn!("Gmail API: Message has no ID");
            }
        }
    } else {
        warn!("Gmail API: No messages found in response");
    }

    let response = EmailResponse {
        count: summaries.len(),
        emails: summaries,
    };

    info!(
        "Gmail API: Completed successfully, returning {} emails",
        response.count
    );
    Ok(serde_json::to_string_pretty(&response)?)
}
