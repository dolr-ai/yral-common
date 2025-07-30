/// Helper trait for getting flow control from environment
pub trait FlowControlFromEnv {
    fn env_prefix(&self) -> &'static str;

    fn get_flow_control_from_env(&self, default: Option<(u32, u32)>) -> Option<(u32, u32)> {
        let rate_key = format!("{}_FLOW_CONTROL_RATE", self.env_prefix());
        let parallel_key = format!("{}_FLOW_CONTROL_PARALLELISM", self.env_prefix());

        let env_rate = std::env::var(&rate_key).ok().and_then(|v| v.parse().ok());
        let env_parallel = std::env::var(&parallel_key)
            .ok()
            .and_then(|v| v.parse().ok());

        default
            .map(|(default_rate, default_parallel)| {
                (
                    env_rate.unwrap_or(default_rate),
                    env_parallel.unwrap_or(default_parallel),
                )
            })
            .or(match (env_rate, env_parallel) {
                (Some(r), Some(p)) => Some((r, p)),
                _ => None,
            })
    }
}
