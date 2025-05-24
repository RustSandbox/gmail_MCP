//! Gmail API Client Library
//!
//! This library exposes a single asynchronous entry point `run` that encapsulates the
//! authentication flow and message-fetching logic previously located in `main.rs`.
//!
//! The implementation is copied verbatim from the original `main.rs`, with only the
//! minimal refactoring required to wrap it inside a function so that it can be reused
//! by any binary target.

mod parser;
pub mod reademail;
pub mod url_remover;

use gmail1::Gmail;
use gmail1::api::ListMessagesResponse;
use gmail1::api::MessagePart;
use gmail1::hyper_rustls::HttpsConnectorBuilder;
use gmail1::hyper_util::client::legacy::Client;
use gmail1::hyper_util::rt::TokioExecutor;
use google_gmail1 as gmail1;
use serde::Deserialize;
use serde::Serialize;
use tracing::{debug, error, info, trace, warn};
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
    trace!(len = data.len(), "Converting bytes to string");
    match String::from_utf8(data.to_vec()) {
        Ok(s) => {
            trace!(len = s.len(), "Successfully converted bytes to string");
            Some(s)
        }
        Err(e) => {
            warn!(error = ?e, "Failed to convert bytes to string: invalid UTF-8");
            None
        }
    }
}

#[tracing::instrument(level = "debug", skip(msg))]
fn extract_body(msg: &gmail1::api::Message) -> String {
    trace!("Starting body extraction from message");
    // First, try top-level body
    if let Some(payload) = &msg.payload {
        trace!("Found message payload");
        if let Some(body) = &payload.body {
            trace!("Found message body");
            if let Some(data) = &body.data {
                trace!("Found body data");
                if let Some(txt) = bytes_to_string(data) {
                    debug!(
                        len = txt.len(),
                        "Successfully extracted body from top-level"
                    );
                    return txt;
                }
            } else {
                trace!("No body data found in top-level body");
            }
        } else {
            trace!("No body found in payload");
        }

        // Recursively search parts for text/plain
        if let Some(parts) = &payload.parts {
            trace!(parts_count = parts.len(), "Searching through message parts");
            if let Some(txt) = find_plain_text(parts) {
                debug!(len = txt.len(), "Successfully extracted body from parts");
                return txt;
            }
        } else {
            trace!("No parts found in payload");
        }
    } else {
        trace!("No payload found in message");
    }
    warn!("No body content found in message");
    String::new()
}

/// Recursively traverse message parts to find the first `text/plain` body.
#[tracing::instrument(level = "trace", skip(parts))]
fn find_plain_text(parts: &[MessagePart]) -> Option<String> {
    trace!(
        parts_count = parts.len(),
        "Searching for plain text in parts"
    );
    for (idx, part) in parts.iter().enumerate() {
        trace!(part_index = idx, mime_type = ?part.mime_type, "Checking part");
        if part.mime_type.as_deref() == Some("text/plain") {
            trace!(part_index = idx, "Found text/plain part");
            if let Some(body) = &part.body {
                trace!(part_index = idx, "Found part body");
                if let Some(data) = &body.data {
                    trace!(part_index = idx, "Found part data");
                    if let Some(txt) = bytes_to_string(data) {
                        debug!(
                            part_index = idx,
                            len = txt.len(),
                            "Successfully extracted text from part"
                        );
                        return Some(txt);
                    }
                } else {
                    trace!(part_index = idx, "No data found in part body");
                }
            } else {
                trace!(part_index = idx, "No body found in part");
            }
        }
        // recurse deeper
        if let Some(sub) = &part.parts {
            trace!(
                part_index = idx,
                sub_parts_count = sub.len(),
                "Recursing into sub-parts"
            );
            if let Some(txt) = find_plain_text(sub) {
                debug!(
                    part_index = idx,
                    len = txt.len(),
                    "Successfully extracted text from sub-parts"
                );
                return Some(txt);
            }
        }
    }
    trace!("No plain text found in any parts");
    None
}

/// Execute the Gmail inbox fetch routine.
///
/// This mirrors the logic that used to live in `main.rs`:
/// 1. Load client credentials
/// 2. Authenticate (OAuth2, token cache, HTTP redirect flow)
/// 3. List the specified number of most recent messages in the user inbox
/// 4. Fetch each message and print basic info (from / subject / snippet)
///
/// # Arguments
/// * `max_results` - The maximum number of emails to fetch. Valid range is 1-500 (Gmail API limit).
///
/// # Errors
/// * I/O errors when reading `client_secret.json`
/// * Authentication or OAuth2 flow failures
/// * Gmail API request errors
#[tracing::instrument(level = "info", skip_all, fields(max_results))]
pub async fn run(max_results: u32) -> Result<String, Box<dyn std::error::Error>> {
    // Validate the max_results parameter against Gmail API limits
    // Gmail API allows a maximum of 500 messages per request
    let max_results = if max_results > 500 {
        error!(
            requested = max_results,
            "Requested more than 500 messages, capping at 500"
        );
        500
    } else if max_results == 0 {
        error!("Requested 0 messages, defaulting to 10");
        10
    } else {
        max_results
    };

    info!("Starting Gmail fetch process");
    debug!("Configuration: max_results = {}", max_results);

    info!("Reading application secret");
    debug!("Loading client_secret.json from disk");
    let secret = yup_oauth2::read_application_secret("client_secret.json")
        .await
        .expect("Failed to read client_secret.json. Please ensure you have downloaded the OAuth 2.0 client credentials (not service account) from Google Cloud Console.");

    // -- credential load successful
    info!("Credentials loaded successfully");
    debug!("Client ID: {}", secret.client_id);
    debug!("Auth URI: {}", secret.auth_uri);
    debug!("Token URI: {}", secret.token_uri);

    info!("Building authenticator");
    debug!("Starting OAuth2 installed flow");
    let scopes = &["https://www.googleapis.com/auth/gmail.readonly"];
    debug!("Using OAuth2 scopes: {:?}", scopes);
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("token_cache.json")
        .build()
        .await
        .expect("Failed to build authenticator. Please check your OAuth configuration.");

    info!("Creating HTTPS connector");
    debug!("Initializing HTTPS connector with native root certificates");
    // Create an HTTPS connector with native root certificates
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .expect("Failed to create HTTPS connector. Please check your system's root certificates.")
        .https_or_http()
        .enable_http1()
        .build();
    debug!("HTTPS connector created successfully");

    info!("Initializing Gmail API client");
    debug!("Creating hyper client with Tokio executor");
    // Create the hyper client with the Tokio executor
    let client = Client::builder(TokioExecutor::new()).build(https);
    debug!("Hyper client created successfully");

    // Initialize the Gmail API hub with the client and authenticator
    let hub = Gmail::new(client, auth);
    info!("Gmail API hub initialized successfully");

    // Gmail hub ready – start fetching messages
    info!(count = max_results, "Listing messages");
    debug!(max_results, "Fetching latest messages from Gmail API");
    let result = hub
        .users()
        .messages_list("me")
        .q("in:inbox")
        .max_results(max_results)
        .doit()
        .await?;

    // Process the results
    match result {
        (
            _,
            ListMessagesResponse {
                messages: Some(messages),
                ..
            },
        ) => {
            info!(count = messages.len(), "Messages retrieved successfully");
            let total_messages = messages.len();
            debug!(total_messages, "Starting to process messages");
            let mut summaries: Vec<EmailSummary> = Vec::new();
            for (idx, message) in messages.into_iter().enumerate() {
                debug!(msg_index = idx, total_messages, "Processing message");
                if let Some(id) = message.id {
                    debug!(%id, "Fetching full message details");
                    match hub
                        .users()
                        .messages_get("me", &id)
                        .format("full")
                        .add_scope("https://www.googleapis.com/auth/gmail.readonly")
                        .doit()
                        .await
                    {
                        Ok((_, msg)) => {
                            debug!(%id, "Message fetched successfully");
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

                                    debug!(%id, subject = %subject, from = %from, "Extracted message headers");
                                    trace!(%id, headers_count = headers.len(), "Processing all headers");

                                    let snippet = msg.snippet.clone().unwrap_or_default();
                                    debug!(%id, snippet_len = snippet.len(), "Extracted message snippet");

                                    let body_raw = extract_body(&msg);
                                    debug!(%id, body_len = body_raw.len(), "Extracted message body");

                                    trace!(%id, "Creating email summary");
                                    summaries.push(EmailSummary {
                                        id: id.clone(),
                                        from,
                                        subject,
                                        snippet,
                                        body_raw,
                                    });
                                    debug!(%id, "Message successfully added to summaries");
                                } else {
                                    warn!(%id, "Message has no headers");
                                }
                            } else {
                                warn!(%id, "Message has no payload");
                            }
                        }
                        Err(e) => {
                            error!(%id, error = ?e, "Failed to fetch message");
                            // If we get a permission denied error, we need to re-authenticate
                            if let gmail1::Error::BadRequest(ref err) = e {
                                if let Some(error) = err.get("error") {
                                    if let Some(status) = error.get("status") {
                                        if status == "PERMISSION_DENIED" {
                                            error!(
                                                "Permission denied. Please ensure you have granted the necessary permissions."
                                            );
                                            error!(
                                                "Try deleting token_cache.json and running the program again."
                                            );
                                            let empty_response = EmailResponse {
                                                emails: vec![],
                                                count: 0,
                                            };
                                            return Ok(serde_json::to_string_pretty(
                                                &empty_response,
                                            )?);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    warn!(msg_index = idx, "Message has no ID");
                }
            }
            // Serialize collected summaries to JSON string
            info!(count = summaries.len(), "Conversion to JSON complete");
            debug!(
                total_bytes = summaries.iter().map(|s| s.body_raw.len()).sum::<usize>(),
                "Total body content size"
            );
            let response = EmailResponse {
                count: summaries.len(),
                emails: summaries,
            };
            let json = serde_json::to_string_pretty(&response)?;
            debug!(bytes = json.len(), "JSON payload size");
            return Ok(json);
        }
        _ => {
            warn!("No messages found in inbox");
            // No messages found – return empty response object
            let empty_response = EmailResponse {
                emails: vec![],
                count: 0,
            };
            return Ok(serde_json::to_string_pretty(&empty_response)?);
        }
    }

    // Should not reach here, but Rust needs a return path
    let empty_response = EmailResponse {
        emails: vec![],
        count: 0,
    };
    Ok(serde_json::to_string_pretty(&empty_response)?)
}
