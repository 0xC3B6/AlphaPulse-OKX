use crate::quality::UniversePolicy;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub coinglass_api_key: Option<String>,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,
    pub require_database: bool,
    pub redis_ttl_secs: u64,
    pub tenant_id: String,
    pub account_id: String,
    pub market_data_max_lag_ms: i64,
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
            database_url: None,
            redis_url: None,
            require_database: false,
            redis_ttl_secs: 30,
            tenant_id: "default".to_string(),
            account_id: "paper".to_string(),
            market_data_max_lag_ms: 5_000,
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
            let value = value.into();
            match key.as_ref() {
                "COINGLASS_API_KEY" => {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        config.coinglass_api_key = Some(trimmed.to_string());
                    }
                }
                "DATABASE_URL" | "ALPHAPULSE_DATABASE_URL" => {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        config.database_url = Some(trimmed.to_string());
                    }
                }
                "REDIS_URL" | "ALPHAPULSE_REDIS_URL" => {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        config.redis_url = Some(trimmed.to_string());
                    }
                }
                "ALPHAPULSE_REQUIRE_DATABASE" => {
                    config.require_database = matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "1" | "true" | "yes" | "on"
                    );
                }
                "ALPHAPULSE_REDIS_TTL_SECS" => {
                    if let Ok(ttl) = value.trim().parse::<u64>() {
                        config.redis_ttl_secs = ttl.max(1);
                    }
                }
                "ALPHAPULSE_TENANT_ID" => {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        config.tenant_id = trimmed.to_string();
                    }
                }
                "ALPHAPULSE_ACCOUNT_ID" => {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        config.account_id = trimmed.to_string();
                    }
                }
                "ALPHAPULSE_MARKET_DATA_MAX_LAG_MS" => {
                    if let Ok(max_lag_ms) = value.trim().parse::<i64>() {
                        config.market_data_max_lag_ms = max_lag_ms.max(0);
                    }
                }
                _ => {}
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
