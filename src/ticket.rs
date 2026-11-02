use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StationTicket {
    pub ws: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npub: Option<String>,
}

impl StationTicket {
    pub fn new(ws: impl Into<String>, name: impl Into<String>, npub: Option<String>) -> Self {
        Self {
            ws: ws.into(),
            name: name.into(),
            npub,
        }
    }

    /// Encode ticket into safe base64 URI (e.g. `deadair://<base64>`)
    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        let b64 = BASE64.encode(json.as_bytes());
        format!("deadair://{}", b64)
    }

    /// Encode ticket into raw base64 string
    pub fn encode_raw_base64(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        BASE64.encode(json.as_bytes())
    }

    /// Attempt to decode a string which might be:
    /// - `deadair://<base64>`
    /// - Raw base64 JSON
    /// - Direct `ws://...` address
    pub fn decode(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let b64_payload = if let Some(stripped) = trimmed.strip_prefix("deadair://") {
            stripped.trim()
        } else {
            trimmed
        };

        // Try decoding as base64 JSON
        if let Ok(bytes) = BASE64.decode(b64_payload.as_bytes()) {
            if let Ok(ticket) = serde_json::from_slice::<StationTicket>(&bytes) {
                if ticket.ws.starts_with("ws://") || ticket.ws.starts_with("wss://") {
                    return Some(ticket);
                }
            }
        }

        // Try raw ws:// parsing if not base64
        if trimmed.starts_with("ws://") || trimmed.starts_with("wss://") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            let ws_url = parts[0].to_string();
            let name = if parts.len() > 1 {
                parts[1].trim_matches(|c| c == '[' || c == ']').to_string()
            } else {
                "station".to_string()
            };
            return Some(StationTicket {
                ws: ws_url,
                name,
                npub: None,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticket_encode_decode() {
        let ticket = StationTicket::new("ws://192.168.1.10:4869", "Alice", Some("npub123".to_string()));
        let encoded = ticket.encode();
        assert!(encoded.starts_with("deadair://"));

        let decoded = StationTicket::decode(&encoded).expect("Failed to decode ticket");
        assert_eq!(decoded.ws, "ws://192.168.1.10:4869");
        assert_eq!(decoded.name, "Alice");
        assert_eq!(decoded.npub, Some("npub123".to_string()));
    }

    #[test]
    fn test_raw_ws_fallback() {
        let decoded = StationTicket::decode("ws://5.1.2.3:4869 [Bob]").expect("Should parse raw ws");
        assert_eq!(decoded.ws, "ws://5.1.2.3:4869");
        assert_eq!(decoded.name, "Bob");
    }
}
