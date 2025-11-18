use crate::config::Config;
use crate::netbox::models::NetBoxSite;

pub struct NetBoxClient {
    config: Config,
    client: reqwest::Client,
}

impl NetBoxClient {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    // Placeholder methods for future NetBox integration
    pub async fn create_site(&self, _site: &NetBoxSite) -> Result<NetBoxSite, anyhow::Error> {
        // TODO: Implement actual NetBox API call
        todo!("NetBox integration not yet implemented")
    }
}

