use crate::server::rate_limiter::XLoadAuthHeaderExtractor;
use governor::middleware::NoOpMiddleware;
use tower_governor::governor::{GovernorConfig, GovernorConfigBuilder};

pub fn get_governor_conf(
    burst_per_minute: u32,
) -> GovernorConfig<XLoadAuthHeaderExtractor, NoOpMiddleware> {
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(60)
        .burst_size(burst_per_minute)
        .key_extractor(XLoadAuthHeaderExtractor)
        .finish()
        .unwrap();

    governor_conf
}
