//! Runtime configuration. Single-operator, local posture — env-driven with
//! sane defaults. No persistence in C1.

#[derive(Clone, Debug)]
pub struct Config {
    /// Address the HTTP server binds to.
    pub bind_addr: String,
    /// Default freshness ceiling applied when a request omits one.
    /// 7 days — code sources gathered live from disk are effectively always
    /// fresh; this only fails closed on genuinely stale inputs.
    pub default_max_source_age_minutes: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8011".to_string(),
            default_max_source_age_minutes: 7 * 24 * 60,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut cfg = Config::default();
        if let Ok(addr) = std::env::var("CONTEXT_RUNTIME_BIND") {
            if !addr.trim().is_empty() {
                cfg.bind_addr = addr;
            }
        }
        if let Ok(age) = std::env::var("CONTEXT_RUNTIME_MAX_SOURCE_AGE_MINUTES") {
            if let Ok(parsed) = age.trim().parse::<u64>() {
                cfg.default_max_source_age_minutes = parsed;
            }
        }
        cfg
    }
}
