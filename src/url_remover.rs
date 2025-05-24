use regex::Regex;
use std::error::Error;

/// Utility for removing URLs from text content
pub struct UrlRemover {
    url_pattern: Regex,
}

impl UrlRemover {
    /// Creates a new UrlRemover instance.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // This regex pattern matches URLs with various protocols and formats
        let url_pattern = Regex::new(
            r"(?i)(https?://|www\.)[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]\.[a-zA-Z]{2,}(?:/[^\s]*)?",
        )?;

        Ok(Self { url_pattern })
    }

    /// Removes all URLs from the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content from which to remove URLs
    ///
    /// # Returns
    ///
    /// A new string with all URLs removed
    ///
    /// # Example
    ///
    /// ```
    /// use gmailrs::url_remover::UrlRemover;
    ///
    /// let remover = UrlRemover::new().unwrap();
    /// let text = "Check out https://example.com and www.test.org";
    /// let cleaned = remover.remove_urls(text);
    /// assert_eq!(cleaned, "Check out  and ");
    /// ```
    pub fn remove_urls(&self, text: &str) -> String {
        self.url_pattern.replace_all(text, "").to_string()
    }

    /// Removes URLs and cleans up the resulting text by:
    /// 1. Removing multiple spaces
    /// 2. Trimming whitespace
    /// 3. Removing empty lines
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to clean
    ///
    /// # Returns
    ///
    /// A cleaned string with URLs removed and formatting improved
    pub fn clean_text(&self, text: &str) -> String {
        let without_urls = self.remove_urls(text);

        // Remove multiple spaces and empty lines
        let without_multiple_spaces = Regex::new(r"\s+")
            .unwrap()
            .replace_all(&without_urls, " ")
            .to_string();

        without_multiple_spaces
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_urls() {
        let remover = UrlRemover::new().unwrap();

        let test_cases = vec![
            ("Check out https://example.com", "Check out "),
            (
                "Visit www.test.org and https://another.com/path",
                "Visit  and ",
            ),
            ("No URLs here", "No URLs here"),
            (
                "Mixed content: https://example.com and some text www.test.org more text",
                "Mixed content:  and some text  more text",
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(remover.remove_urls(input), expected);
        }
    }

    #[test]
    fn test_clean_text() {
        let remover = UrlRemover::new().unwrap();

        let input = "Check out https://example.com\n\n   Visit www.test.org\n\n\nSome text here";
        let expected = "Check out Visit Some text here";

        assert_eq!(remover.clean_text(input), expected);
    }
}
