//! Example: Fetch a Specific Number of Emails from Gmail
//!
//! This example demonstrates how to use the gmailrs library to fetch
//! a configurable number of emails from your Gmail inbox.
//!
//! Before running this example, ensure you have:
//! 1. Downloaded your OAuth 2.0 client credentials from Google Cloud Console
//! 2. Saved them as `client_secret.json` in the project root
//! 3. Enabled the Gmail API for your project

use gmailrs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments to get the number of emails to fetch
    let args: Vec<String> = std::env::args().collect();

    // Default to 10 emails if no argument is provided
    let max_results: u32 = if args.len() > 1 {
        args[1].parse().unwrap_or_else(|_| {
            eprintln!("Invalid number provided. Using default of 10.");
            10
        })
    } else {
        println!("No number specified. Fetching 10 emails by default.");
        println!("Usage: cargo run --example fetch_emails <number_of_emails>");
        10
    };

    // Ensure the number is within Gmail API limits (1-500)
    let max_results = max_results.clamp(1, 500);

    println!("Fetching {} email(s) from Gmail...", max_results);

    // Call the library function to fetch emails
    match gmailrs::run(max_results).await {
        Ok(json_result) => {
            // Parse the JSON result to count emails
            if let Ok(response) = serde_json::from_str::<gmailrs::EmailResponse>(&json_result) {
                println!("\nSuccessfully fetched {} email(s):", response.count);
                println!("{}", "-".repeat(60));

                // Display a summary of each email
                for (idx, email) in response.emails.iter().enumerate() {
                    println!("\n{}. From: {}", idx + 1, email.from);
                    println!("   Subject: {}", email.subject);
                    println!(
                        "   Preview: {}...",
                        email.snippet.chars().take(50).collect::<String>()
                    );
                }
            } else {
                // If JSON parsing fails, just print the raw result
                println!("Raw result:\n{}", json_result);
            }
        }
        Err(e) => {
            eprintln!("Error fetching emails: {}", e);
            eprintln!("\nTroubleshooting tips:");
            eprintln!("1. Ensure client_secret.json exists in the project root");
            eprintln!("2. Check that the Gmail API is enabled in Google Cloud Console");
            eprintln!("3. Try deleting token_cache.json if authentication fails");
        }
    }

    Ok(())
}
