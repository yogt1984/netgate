use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("netgate=debug".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

