/// Configuration for NATS JetStream relay
pub struct RelayConfig {
    pub nats_url: String,
    pub read_only: bool,
}
