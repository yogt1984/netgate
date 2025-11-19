use serde::{Deserialize, Serialize};

/// NetBox API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetBoxResponse<T> {
    pub count: Option<i32>,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Option<Vec<T>>,
}

/// NetBox Site model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetBoxSite {
    pub id: Option<i32>,
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub status: Option<SiteStatus>,
    pub region: Option<i32>,
    pub tenant: Option<i32>,
    pub facility: Option<String>,
    pub physical_address: Option<String>,
    pub shipping_address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub comments: Option<String>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
    pub created: Option<String>,
    pub last_updated: Option<String>,
}

/// NetBox Site Status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SiteStatus {
    Active,
    Planned,
    Retired,
    Staging,
}

/// NetBox Device model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetBoxDevice {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub device_type: Option<i32>,
    pub device_role: Option<i32>,
    pub tenant: Option<i32>,
    pub platform: Option<i32>,
    pub serial: Option<String>,
    pub asset_tag: Option<String>,
    pub site: Option<i32>,
    pub location: Option<i32>,
    pub rack: Option<i32>,
    pub position: Option<f64>,
    pub face: Option<DeviceFace>,
    pub status: Option<DeviceStatus>,
    pub primary_ip4: Option<i32>,
    pub primary_ip6: Option<i32>,
    pub cluster: Option<i32>,
    pub virtual_chassis: Option<i32>,
    pub vc_position: Option<i32>,
    pub vc_priority: Option<i32>,
    pub comments: Option<String>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
    pub created: Option<String>,
    pub last_updated: Option<String>,
}

/// NetBox Device Face
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceFace {
    Front,
    Rear,
}

/// NetBox Device Status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceStatus {
    Offline,
    Active,
    Planned,
    Staged,
    Failed,
    Inventory,
    Decommissioning,
}

/// Request payload for creating a site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSiteRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub status: Option<SiteStatus>,
    pub region: Option<i32>,
    pub tenant: Option<i32>,
    pub facility: Option<String>,
    pub physical_address: Option<String>,
    pub shipping_address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub comments: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Request payload for updating a site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSiteRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub status: Option<SiteStatus>,
    pub region: Option<i32>,
    pub tenant: Option<i32>,
    pub facility: Option<String>,
    pub physical_address: Option<String>,
    pub shipping_address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub comments: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Request payload for creating a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: Option<String>,
    pub device_type: i32,
    pub device_role: i32,
    pub tenant: Option<i32>,
    pub platform: Option<i32>,
    pub serial: Option<String>,
    pub asset_tag: Option<String>,
    pub site: i32,
    pub location: Option<i32>,
    pub rack: Option<i32>,
    pub position: Option<f64>,
    pub face: Option<DeviceFace>,
    pub status: Option<DeviceStatus>,
    pub cluster: Option<i32>,
    pub comments: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Request payload for updating a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub device_type: Option<i32>,
    pub device_role: Option<i32>,
    pub tenant: Option<i32>,
    pub platform: Option<i32>,
    pub serial: Option<String>,
    pub asset_tag: Option<String>,
    pub site: Option<i32>,
    pub location: Option<i32>,
    pub rack: Option<i32>,
    pub position: Option<f64>,
    pub face: Option<DeviceFace>,
    pub status: Option<DeviceStatus>,
    pub cluster: Option<i32>,
    pub comments: Option<String>,
    pub tags: Option<Vec<String>>,
}

