use serde::{Deserialize, Serialize};

// Placeholder for NetBox models
// Will be implemented when integrating with NetBox API

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetBoxSite {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

