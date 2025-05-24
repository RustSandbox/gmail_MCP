# Gmail API Client in Rust

A simple Rust client for fetching Gmail emails with OAuth2 authentication.

## Features

- OAuth2 authentication with Google
- Fetch emails with configurable count (1-500)
- HTML to text conversion
- URL removal from email bodies
- MCP server support
- JSON output format

## Prerequisites

- Rust 1.70+
- Google Cloud Platform account with Gmail API enabled
- OAuth 2.0 client credentials

## Setup

1. **Google Cloud Console**:
   - Enable Gmail API
   - Create OAuth 2.0 credentials (Desktop application)
   - Download as `client_secret.json` in project root

2. **Build and run**:
   ```bash
   cargo build
   cargo run
   ```

## Usage

### As a Library

```rust
use gmailrs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let emails_json = gmailrs::run(10).await?;
    let response: gmailrs::EmailResponse = serde_json::from_str(&emails_json)?;
    
    for email in response.emails {
        println!("From: {}, Subject: {}", email.from, email.subject);
    }
    
    Ok(())
}
```

### As MCP Server

```bash
cargo run
# Server runs on http://0.0.0.0:3003/sse
```

## Response Format

```json
{
  "emails": [
    {
      "id": "18f123456789abcd",
      "from": "sender@example.com", 
      "subject": "Email Subject",
      "snippet": "Email preview...",
      "body_raw": "Clean email body (URLs removed)"
    }
  ],
  "count": 10
}
```

## License

MIT License 