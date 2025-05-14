//! Gmail API Client Library
//!
//! This library exposes a single asynchronous entry point `run` that encapsulates the
//! authentication flow and message-fetching logic previously located in `main.rs`.
//!
//! The implementation is copied verbatim from the original `main.rs`, with only the
//! minimal refactoring required to wrap it inside a function so that it can be reused
//! by any binary target.

use gmail1::Gmail;
use gmail1::api::ListMessagesResponse;
use gmail1::hyper_rustls::HttpsConnectorBuilder;
use gmail1::hyper_util::client::legacy::Client;
use gmail1::hyper_util::rt::TokioExecutor;
use google_gmail1 as gmail1;
use serde::Serialize;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

/// Lightweight representation of an email message that our API returns.
#[derive(Serialize)]
pub struct EmailSummary {
    /// The unique Gmail message ID.
    pub id: String,
    /// The value of the `From` header.
    pub from: String,
    /// The value of the `Subject` header.
    pub subject: String,
    /// A short snippet of the message body.
    pub snippet: String,
}

/// Execute the Gmail inbox fetch routine.
///
/// This mirrors the logic that used to live in `main.rs`:
/// 1. Load client credentials
/// 2. Authenticate (OAuth2, token cache, HTTP redirect flow)
/// 3. List the 10 most recent messages in the user inbox
/// 4. Fetch each message and print basic info (from / subject / snippet)
///
/// # Errors
/// * I/O errors when reading `client_secret.json`
/// * Authentication or OAuth2 flow failures
/// * Gmail API request errors
pub async fn run() -> Result<String, Box<dyn std::error::Error>> {
    // For library use we avoid println-noise; caller can log if desired.

    // Load OAuth 2.0 credentials from the client secret file
    let secret = yup_oauth2::read_application_secret("client_secret.json")
        .await
        .expect("Failed to read client_secret.json. Please ensure you have downloaded the OAuth 2.0 client credentials (not service account) from Google Cloud Console.");

    // -- credential load successful

    // Set up the authenticator with HTTP redirect method and token storage
    let scopes = &["https://www.googleapis.com/auth/gmail.readonly"];
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("token_cache.json")
        .build()
        .await
        .expect("Failed to build authenticator. Please check your OAuth configuration.");

    // Create an HTTPS connector with native root certificates
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .expect("Failed to create HTTPS connector. Please check your system's root certificates.")
        .https_or_http()
        .enable_http1()
        .build();

    // Create the hyper client with the Tokio executor
    let client = Client::builder(TokioExecutor::new()).build(https);

    // Initialize the Gmail API hub with the client and authenticator
    let hub = Gmail::new(client, auth);

    // Gmail hub ready – start fetching messages

    // List messages in the user's inbox with a maximum of 10 results
    let result = hub
        .users()
        .messages_list("me")
        .q("in:inbox")
        .max_results(10)
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
            let mut summaries: Vec<EmailSummary> = Vec::new();
            for message in messages {
                if let Some(id) = message.id {
                    println!("Fetching details for message ID: {}", id);
                    // Fetch the full message details with explicit scope
                    match hub
                        .users()
                        .messages_get("me", &id)
                        .add_scope("https://www.googleapis.com/auth/gmail.readonly")
                        .doit()
                        .await
                    {
                        Ok((_, msg)) => {
                            if let Some(payload) = msg.payload {
                                if let Some(headers) = payload.headers {
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

                                    // Take snippet once to avoid move issues
                                    let snippet = msg.snippet.unwrap_or_default();

                                    // Optional: retain debug logging
                                    // println!("From: {}", from);
                                    // println!("Subject: {}", subject);
                                    // println!("Snippet: {}", snippet);
                                    // println!("-----------------------------------");

                                    summaries.push(EmailSummary {
                                        id: id.clone(),
                                        from,
                                        subject,
                                        snippet,
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error fetching message {}: {:?}", id, e);
                            // If we get a permission denied error, we need to re-authenticate
                            if let gmail1::Error::BadRequest(ref err) = e {
                                if let Some(error) = err.get("error") {
                                    if let Some(status) = error.get("status") {
                                        if status == "PERMISSION_DENIED" {
                                            println!(
                                                "Permission denied. Please ensure you have granted the necessary permissions."
                                            );
                                            println!(
                                                "Try deleting token_cache.json and running the program again."
                                            );
                                            return Ok("[]".to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Serialize collected summaries to JSON string
            let json = serde_json::to_string_pretty(&summaries)?;
            return Ok(json);
        }
        _ => {
            // No messages found – return empty JSON array
            return Ok("[]".to_string());
        }
    }

    // Should not reach here, but Rust needs a return path
    Ok("[]".to_string())
}
