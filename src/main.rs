//! Minimal binary that delegates all Gmail logic to the library.

/// This binary exists solely to bootstrap a Tokio runtime and call the
/// `gmailrs::run` function provided by the library (see `src/lib.rs`).
/// All Gmail-related logic, authentication, and API calls live in the library.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = gmailrs::run().await?;
    println!("{json}");
    Ok(())
}
