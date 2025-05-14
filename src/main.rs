//! Gmail API Client
//!
//! This crate provides a simple interface to interact with the Gmail API using Rust.
//! It allows you to authenticate with Google's OAuth2 system and perform basic Gmail operations
//! such as listing and reading messages.
//!
//! # Features
//!
//! - OAuth2 authentication with Google
//! - Token persistence for automatic re-authentication
//! - Message listing and retrieval
//! - Detailed message information extraction
//!
//! # Prerequisites
//!
//! 1. A Google Cloud Platform account
//! 2. A project with the Gmail API enabled
//! 3. OAuth 2.0 client credentials (not service account)
//!
//! # Setup
//!
//! 1. Go to the [Google Cloud Console](https://console.cloud.google.com)
//! 2. Create a new project or select an existing one
//! 3. Enable the Gmail API
//! 4. Create OAuth 2.0 credentials (Desktop application)
//! 5. Download the credentials and save as `client_secret.json`
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```no_run
//! use gmail1::Gmail;
//! use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load credentials and authenticate
//!     let secret = yup_oauth2::read_application_secret("client_secret.json").await?;
//!     let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
//!         .persist_tokens_to_disk("token_cache.json")
//!         .build()
//!         .await?;
//!     
//!     // Initialize Gmail client and fetch messages
//!     let hub = Gmail::new(client, auth);
//!     let result = hub.users().messages_list("me").max_results(10).doit().await?;
//!     // Process messages...
//!     Ok(())
//! }
//! ```
//!
//! # Authentication
//!
//! The client uses OAuth2 for authentication. The first time you run the application:
//! 1. A browser window will open
//! 2. You'll be asked to sign in to your Google account
//! 3. You'll need to grant permission to access your Gmail
//! 4. The tokens will be saved to `token_cache.json` for future use
//!
//! # Error Handling
//!
//! The client uses Rust's Result type for error handling. Common errors include:
//! - Authentication failures
//! - Missing or invalid credentials
//! - Network errors
//! - API rate limiting
//!
//! # Security Considerations
//!
//! - Never commit `client_secret.json` or `token_cache.json` to version control
//! - Keep your OAuth credentials secure
//! - Regularly rotate your credentials
//! - Use appropriate scopes for your application

use gmail1::Gmail;
use gmail1::api::ListMessagesResponse;
use gmail1::hyper_rustls::HttpsConnectorBuilder;
use gmail1::hyper_util::client::legacy::Client;
use gmail1::hyper_util::rt::TokioExecutor;
use google_gmail1 as gmail1;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

/// Main entry point for the Gmail API client.
///
/// This function demonstrates how to:
/// 1. Authenticate with Google's OAuth2 system
/// 2. Initialize the Gmail API client
/// 3. List messages from the user's inbox
/// 4. Fetch and display message details
///
/// # Errors
///
/// This function can return errors in several cases:
/// * Failed to read client secret file
/// * Authentication failures
/// * API request failures
/// * Network errors
///
/// # Examples
///
/// ```no_run
/// # use gmail1::Gmail;
/// # use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Your authentication and API calls here
/// #     Ok(())
/// # }
/// ```
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Gmail API client...");

    // Load OAuth 2.0 credentials from the client secret file
    let secret = yup_oauth2::read_application_secret("client_secret.json")
        .await
        .expect("Failed to read client_secret.json. Please ensure you have downloaded the OAuth 2.0 client credentials (not service account) from Google Cloud Console.");

    println!("Successfully loaded OAuth credentials.");

    // Set up the authenticator with HTTP redirect method and token storage
    let scopes = &["https://www.googleapis.com/auth/gmail.readonly"];
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("token_cache.json")
        .build()
        .await
        .expect("Failed to build authenticator. Please check your OAuth configuration.");

    println!("Successfully initialized authenticator.");

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

    println!("Successfully initialized Gmail API client.");
    println!("Attempting to fetch messages from your inbox...");

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
            println!("Found {} messages in your inbox.", messages.len());
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

                                    println!("From: {}", from);
                                    println!("Subject: {}", subject);
                                    println!("Snippet: {}", msg.snippet.unwrap_or_default());
                                    println!("-----------------------------------");
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
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => println!("No messages found in your inbox."),
    }

    println!("Gmail API client finished successfully.");
    Ok(())
}
