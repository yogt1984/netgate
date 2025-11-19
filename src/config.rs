pub struct Config {
    pub port: u16,
    pub netbox_url: String,
    pub netbox_token: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            netbox_url: std::env::var("NETBOX_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            netbox_token: std::env::var("NETBOX_TOKEN")
                .unwrap_or_else(|_| "".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // Save original values
        let orig_port = std::env::var("PORT").ok();
        let orig_url = std::env::var("NETBOX_URL").ok();
        let orig_token = std::env::var("NETBOX_TOKEN").ok();

        // Clear env vars for this test
        std::env::remove_var("PORT");
        std::env::remove_var("NETBOX_URL");
        std::env::remove_var("NETBOX_TOKEN");

        let config = Config::from_env();

        assert_eq!(config.port, 8080);
        assert_eq!(config.netbox_url, "http://localhost:8000");
        assert_eq!(config.netbox_token, "");

        // Restore original values
        if let Some(val) = orig_port {
            std::env::set_var("PORT", val);
        } else {
            std::env::remove_var("PORT");
        }
        if let Some(val) = orig_url {
            std::env::set_var("NETBOX_URL", val);
        } else {
            std::env::remove_var("NETBOX_URL");
        }
        if let Some(val) = orig_token {
            std::env::set_var("NETBOX_TOKEN", val);
        } else {
            std::env::remove_var("NETBOX_TOKEN");
        }
    }

    #[test]
    fn test_config_from_env() {
        // Save original values
        let orig_port = std::env::var("PORT").ok();
        let orig_url = std::env::var("NETBOX_URL").ok();
        let orig_token = std::env::var("NETBOX_TOKEN").ok();

        std::env::set_var("PORT", "9090");
        std::env::set_var("NETBOX_URL", "http://netbox.example.com");
        std::env::set_var("NETBOX_TOKEN", "test-token");

        let config = Config::from_env();

        assert_eq!(config.port, 9090);
        assert_eq!(config.netbox_url, "http://netbox.example.com");
        assert_eq!(config.netbox_token, "test-token");

        // Restore original values
        if let Some(val) = orig_port {
            std::env::set_var("PORT", val);
        } else {
            std::env::remove_var("PORT");
        }
        if let Some(val) = orig_url {
            std::env::set_var("NETBOX_URL", val);
        } else {
            std::env::remove_var("NETBOX_URL");
        }
        if let Some(val) = orig_token {
            std::env::set_var("NETBOX_TOKEN", val);
        } else {
            std::env::remove_var("NETBOX_TOKEN");
        }
    }

    #[test]
    fn test_config_invalid_port() {
        // Save original values
        let orig_port = std::env::var("PORT").ok();
        let orig_url = std::env::var("NETBOX_URL").ok();
        let orig_token = std::env::var("NETBOX_TOKEN").ok();

        std::env::set_var("PORT", "invalid");
        std::env::remove_var("NETBOX_URL");
        std::env::remove_var("NETBOX_TOKEN");

        let config = Config::from_env();

        // Should fall back to default
        assert_eq!(config.port, 8080);

        // Restore original values
        if let Some(val) = orig_port {
            std::env::set_var("PORT", val);
        } else {
            std::env::remove_var("PORT");
        }
        if let Some(val) = orig_url {
            std::env::set_var("NETBOX_URL", val);
        } else {
            std::env::remove_var("NETBOX_URL");
        }
        if let Some(val) = orig_token {
            std::env::set_var("NETBOX_TOKEN", val);
        } else {
            std::env::remove_var("NETBOX_TOKEN");
        }
    }
}

