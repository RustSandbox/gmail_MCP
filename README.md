# Gmail API Client in Rust

A Rust client for interacting with the Gmail API, providing a simple and efficient way to access Gmail functionality programmatically.

## Features

- 🔐 OAuth2 authentication with Google
- 💾 Token persistence for automatic re-authentication
- 📧 Message listing and retrieval
- 📝 Detailed message information extraction
- 🛡️ Secure credential handling
- ⚡ Async/await support

## Prerequisites

- Rust 1.70 or later
- Cargo (Rust's package manager)
- A Google Cloud Platform account
- A project with the Gmail API enabled
- OAuth 2.0 client credentials

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/gmailrs.git
   cd gmailrs
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run the application:
   ```bash
   cargo run
   ```

## Setup

### Google Cloud Console Setup

1. Go to the [Google Cloud Console](https://console.cloud.google.com)
2. Create a new project or select an existing one
3. Enable the Gmail API:
   - Navigate to "APIs & Services" > "Library"
   - Search for "Gmail API"
   - Click "Enable"
4. Create OAuth 2.0 credentials:
   - Go to "APIs & Services" > "Credentials"
   - Click "Create Credentials" > "OAuth client ID"
   - Choose "Desktop application" as the application type
   - Download the credentials and save as `client_secret.json` in the project root

### First Run

When you run the application for the first time:
1. A browser window will open
2. Sign in to your Google account
3. Grant permission to access your Gmail
4. The tokens will be saved to `token_cache.json` for future use

## Usage

```rust
use gmail1::Gmail;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load credentials and authenticate
    let secret = yup_oauth2::read_application_secret("client_secret.json").await?;
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("token_cache.json")
        .build()
        .await?;
    
    // Initialize Gmail client and fetch messages
    let hub = Gmail::new(client, auth);
    let result = hub.users().messages_list("me").max_results(10).doit().await?;
    // Process messages...
    Ok(())
}
```

## Documentation

Generate local documentation:
```bash
cargo doc --open
```

## Dependencies

- `google-gmail1`: Gmail API client
- `yup-oauth2`: OAuth2 authentication
- `tokio`: Async runtime
- `hyper`: HTTP client
- `serde`: Serialization/deserialization

## Error Handling

The client uses Rust's Result type for error handling. Common errors include:
- Authentication failures
- Missing or invalid credentials
- Network errors
- API rate limiting

## Security

- Never commit `client_secret.json` or `token_cache.json` to version control
- Keep your OAuth credentials secure
- Regularly rotate your credentials
- Use appropriate scopes for your application

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Google Gmail API](https://developers.google.com/gmail/api)
- [Rust Programming Language](https://www.rust-lang.org)
- [yup-oauth2](https://crates.io/crates/yup-oauth2) 