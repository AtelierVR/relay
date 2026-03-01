/// Moderation record for a specific (userId, address) pair within an instance.
#[derive(Debug, Clone)]
pub struct UserModerated {
    pub user_id: u32,
    pub address: String,
    pub is_blacklisted: bool,
    pub is_whitelisted: bool,
}

impl UserModerated {
    pub fn new(user_id: u32, address: impl Into<String>) -> Self {
        Self {
            user_id,
            address: address.into(),
            is_blacklisted: false,
            is_whitelisted: false,
        }
    }
}
