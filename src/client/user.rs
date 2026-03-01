/// Authenticated user attached to a client connection.
#[derive(Debug, Clone)]
pub struct User {
    /// Master-server assigned numeric user ID (0 is invalid / unauthenticated).
    pub id: u32,
    /// The user's login name.
    pub username: String,
    /// The user's display name.
    pub display_name: String,
    /// The origin server address (e.g. `"nox.example.com"`).
    pub address: String,
}

impl User {
    pub const INVALID_ID: u32 = 0;

    /// `"{id}@{address}"`
    pub fn to_identifier(&self) -> String {
        format!("{}@{}", self.id, self.address)
    }

    pub fn is_valid(&self) -> bool {
        self.id != Self::INVALID_ID
    }
}
