/// Maximum message size allowed by Meshtastic protocol (in bytes)
const MAX_MESSAGE_BYTES: usize = 200;

/// Suffix added to truncated messages
const TRUNCATION_SUFFIX: &str = "... (truncated)";

/// Truncates a message to fit within Meshtastic's 200-byte limit.
///
/// If the message is already under the limit, it's returned unchanged.
/// If over the limit, it's truncated at a UTF-8 character boundary and
/// a "... (truncated)" suffix is appended.
///
/// # Examples
///
/// ```
/// use mesh_jawn::utils::bounded_message::truncate_message;
///
/// let short = "Hello";
/// assert_eq!(truncate_message(short), "Hello");
///
/// let long = "a".repeat(250);
/// let truncated = truncate_message(&long);
/// assert!(truncated.len() <= 200);
/// assert!(truncated.ends_with("... (truncated)"));
/// ```
pub fn truncate_message(message: &str) -> String {
    let message_bytes = message.len();

    // If message fits, return as-is
    if message_bytes <= MAX_MESSAGE_BYTES {
        return message.to_string();
    }

    // Calculate how much content we can keep
    let suffix_bytes = TRUNCATION_SUFFIX.len();
    let max_content_bytes = MAX_MESSAGE_BYTES - suffix_bytes;

    // Find the UTF-8 character boundary at or before max_content_bytes
    let mut truncate_at = max_content_bytes;
    while truncate_at > 0 && !message.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }

    // Build truncated message
    format!("{}{}", &message[..truncate_at], TRUNCATION_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_message_unchanged() {
        let msg = "Hello, world!";
        assert_eq!(truncate_message(msg), msg);
    }

    #[test]
    fn exactly_200_bytes_unchanged() {
        let msg = "a".repeat(200);
        assert_eq!(truncate_message(&msg), msg);
    }

    #[test]
    fn long_message_truncated() {
        let msg = "a".repeat(250);
        let truncated = truncate_message(&msg);

        // Should be exactly 200 bytes
        assert_eq!(truncated.as_bytes().len(), MAX_MESSAGE_BYTES);

        // Should end with suffix
        assert!(truncated.ends_with(TRUNCATION_SUFFIX));
    }

    #[test]
    fn utf8_safety_with_emoji() {
        // Create a message with emoji (4 bytes each) that would split mid-character
        // 🦀 is 4 bytes in UTF-8
        let msg = "🦀".repeat(60); // 240 bytes total

        let truncated = truncate_message(&msg);

        // Should not panic and should be valid UTF-8
        assert!(truncated.as_bytes().len() <= MAX_MESSAGE_BYTES);
        assert!(truncated.ends_with(TRUNCATION_SUFFIX));

        // Verify it's valid UTF-8 by checking we can iterate chars
        let _chars: Vec<char> = truncated.chars().collect();
    }

    #[test]
    fn utf8_safety_with_multibyte_chars() {
        // Mix of 1-byte (a), 2-byte (é), 3-byte (€), and 4-byte (🦀) chars
        let msg = format!(
            "{}{}{}{}",
            "a".repeat(50),
            "é".repeat(30),
            "€".repeat(20),
            "🦀".repeat(15),
        );

        let truncated = truncate_message(&msg);

        // Should be at or under limit
        assert!(truncated.as_bytes().len() <= MAX_MESSAGE_BYTES);

        // Should be valid UTF-8
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }

    #[test]
    fn empty_message() {
        let msg = "";
        assert_eq!(truncate_message(msg), "");
    }

    #[test]
    fn exactly_at_boundary_after_suffix() {
        // Create message that's exactly 201 bytes
        let msg = "a".repeat(201);
        let truncated = truncate_message(&msg);

        assert_eq!(truncated.as_bytes().len(), MAX_MESSAGE_BYTES);
        assert!(truncated.ends_with(TRUNCATION_SUFFIX));
    }
}
