use std::sync::OnceLock;

pub mod typed_prometheus;

pub fn metrics_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("SPACETIMEDB_DISABLE_METRICS")
            .map(|v| {
                let v = v.trim();
                !(v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
            })
            .unwrap_or(true)
    })
}
