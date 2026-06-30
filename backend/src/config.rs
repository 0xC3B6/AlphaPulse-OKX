use crate::quality::UniversePolicy;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub coinglass_api_key: Option<String>,
    pub scan_interval_secs: u64,
    pub dynamic_pool_size: usize,
    pub trend_alert_threshold: u8,
    pub range_alert_threshold: u8,
    pub watch_threshold: u8,
    pub min_listing_age_days: f64,
    pub new_listing_days: f64,
    pub min_history_days: f64,
    pub thin_history_days: f64,
    pub fixed_watchlist: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8787,
            coinglass_api_key: None,
            scan_interval_secs: 30,
            dynamic_pool_size: 40,
            trend_alert_threshold: 80,
            range_alert_threshold: 80,
            watch_threshold: 65,
            min_listing_age_days: 3.0,
            new_listing_days: 14.0,
            min_history_days: 3.0,
            thin_history_days: 7.0,
            fixed_watchlist: vec![
                "BTC-USDT-SWAP",
                "ETH-USDT-SWAP",
                "SOL-USDT-SWAP",
                "XRP-USDT-SWAP",
                "DOGE-USDT-SWAP",
                "LAB-USDT-SWAP",
                "RAVE-USDT-SWAP",
                "BEAT-USDT-SWAP",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let _ = dotenvy::from_filename(".env.local");
        let _ = dotenvy::dotenv();
        Self::from_env()
    }

    pub fn from_env() -> Self {
        Self::from_env_pairs(std::env::vars())
    }

    pub fn from_env_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        let mut config = Self::default();
        for (key, value) in pairs {
            if key.as_ref() == "COINGLASS_API_KEY" {
                let value = value.into();
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    config.coinglass_api_key = Some(trimmed.to_string());
                }
            }
        }
        config
    }

    pub fn universe_policy(&self) -> UniversePolicy {
        UniversePolicy {
            min_listing_age_days: self.min_listing_age_days,
            new_listing_days: self.new_listing_days,
            min_history_days: self.min_history_days,
            thin_history_days: self.thin_history_days,
        }
    }
}
