#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub scan_interval_secs: u64,
    pub dynamic_pool_size: usize,
    pub trend_alert_threshold: u8,
    pub range_alert_threshold: u8,
    pub watch_threshold: u8,
    pub fixed_watchlist: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8787,
            scan_interval_secs: 30,
            dynamic_pool_size: 40,
            trend_alert_threshold: 80,
            range_alert_threshold: 80,
            watch_threshold: 65,
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
