/// Specifies where a command response should be sent
#[derive(Debug, Clone, PartialEq)]
pub enum ReplyDestination {
    /// Reply to the sender via DM
    Sender,
    /// Reply via broadcast (if applicable)
    Broadcast,
}

/// Response data returned by command handlers
///
/// Commands return this struct containing the message(s) to send
/// and where to send them. The event loop handles the actual I/O.
#[derive(Debug, Clone)]
pub struct CommandResponse {
    /// Lines of text to send (each will be truncated to 200 bytes if needed)
    pub messages: Vec<String>,
    /// Where to send the response
    pub destination: ReplyDestination,
}

impl CommandResponse {
    /// Create a new response with a single message to the sender
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            messages: vec![message.into()],
            destination: ReplyDestination::Sender,
        }
    }

    /// Create a response with multiple messages to the sender
    pub fn new_multi(messages: Vec<String>) -> Self {
        Self {
            messages,
            destination: ReplyDestination::Sender,
        }
    }

    /// Create a broadcast response
    pub fn broadcast(message: impl Into<String>) -> Self {
        Self {
            messages: vec![message.into()],
            destination: ReplyDestination::Broadcast,
        }
    }
}

// Ergonomic conversions: allow commands to return simple types that convert to CommandResponse

/// Convert a single string to a CommandResponse (to sender)
impl From<String> for CommandResponse {
    fn from(message: String) -> Self {
        CommandResponse::new(message)
    }
}

/// Convert a &str to a CommandResponse (to sender)
impl From<&str> for CommandResponse {
    fn from(message: &str) -> Self {
        CommandResponse::new(message.to_string())
    }
}

/// Convert a Vec<String> to a CommandResponse (to sender)
impl From<Vec<String>> for CommandResponse {
    fn from(messages: Vec<String>) -> Self {
        CommandResponse::new_multi(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let response: CommandResponse = "Hello".into();
        assert_eq!(response.messages, vec!["Hello"]);
        assert_eq!(response.destination, ReplyDestination::Sender);
    }

    #[test]
    fn test_from_string() {
        let response: CommandResponse = String::from("Hello").into();
        assert_eq!(response.messages, vec!["Hello"]);
        assert_eq!(response.destination, ReplyDestination::Sender);
    }

    #[test]
    fn test_from_vec() {
        let messages = vec!["Line 1".to_string(), "Line 2".to_string()];
        let response: CommandResponse = messages.clone().into();
        assert_eq!(response.messages, messages);
        assert_eq!(response.destination, ReplyDestination::Sender);
    }

    #[test]
    fn test_broadcast() {
        let response = CommandResponse::broadcast("Announcement");
        assert_eq!(response.messages, vec!["Announcement"]);
        assert_eq!(response.destination, ReplyDestination::Broadcast);
    }
}
