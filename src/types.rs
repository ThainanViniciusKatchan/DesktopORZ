use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconEntry {
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopProfile {
    pub profile_name: String,
    pub resolution: Resolution,
    pub timestamp_utc: u64,
    pub icons: Vec<IconEntry>,
    /// Layouts específicos por resolução, indexados por "LARGURAxALTURA".
    #[serde(default)]
    pub resolutions: std::collections::HashMap<String, Vec<IconEntry>>,
}
