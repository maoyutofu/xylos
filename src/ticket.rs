use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rand_core::{OsRng, RngCore};

#[derive(Debug, Clone)]
pub struct DownloadTicket {
    pub username: String,
    pub href_path: String,
    pub expires_at: Instant,
}

pub struct TicketStore {
    ttl: Duration,
    tickets: Mutex<HashMap<String, DownloadTicket>>,
}

impl TicketStore {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            tickets: Mutex::new(HashMap::new()),
        }
    }

    pub fn issue(&self, username: &str, href_path: &str) -> String {
        self.issue_with_ttl(username, href_path, self.ttl)
    }

    pub fn issue_with_ttl(&self, username: &str, href_path: &str, ttl: Duration) -> String {
        let mut bytes = [0u8; 16];
        OsRng.fill_bytes(&mut bytes);
        let ticket: String = bytes.iter().map(|b| format!("{b:02x}")).collect();

        let expires_at = Instant::now() + ttl;
        let mut tickets = self
            .tickets
            .lock()
            .expect("ticket store lock should not poison");
        let now = Instant::now();
        tickets.retain(|_, item| item.expires_at > now);
        tickets.insert(
            ticket.clone(),
            DownloadTicket {
                username: username.to_string(),
                href_path: href_path.to_string(),
                expires_at,
            },
        );
        ticket
    }

    pub fn validate(&self, ticket: &str) -> Option<(String, String)> {
        let now = Instant::now();
        let mut tickets = self
            .tickets
            .lock()
            .expect("ticket store lock should not poison");
        tickets.retain(|_, item| item.expires_at > now);
        tickets
            .get(ticket)
            .filter(|item| item.expires_at > now)
            .map(|item| (item.username.clone(), item.href_path.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issues_and_validates_ticket() {
        let store = TicketStore::new(60);
        let ticket = store.issue("alice", "/documents/file.zip");
        assert_eq!(ticket.len(), 32);

        let validated = store.validate(&ticket);
        assert_eq!(
            validated,
            Some(("alice".to_string(), "/documents/file.zip".to_string()))
        );

        assert_eq!(store.validate("invalid_ticket"), None);
    }

    #[test]
    fn expires_ticket() {
        let store = TicketStore::new(0);
        let ticket = store.issue("bob", "/data.tar");
        // With 0 TTL, it expires immediately
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(store.validate(&ticket), None);
    }
}
