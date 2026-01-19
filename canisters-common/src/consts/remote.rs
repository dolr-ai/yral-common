use std::sync::LazyLock;
use url::Url;

pub static METADATA_API_BASE: LazyLock<Url> =
    LazyLock::new(|| Url::parse("https://metadata.yral.com").unwrap());

pub const AGENT_URL: &str = "https://ic0.app";

pub static PUMP_AND_DUMP_WORKER_URL: LazyLock<Url> =
    LazyLock::new(|| Url::parse("https://yral-pump-n-dump.go-bazzinga.workers.dev/").unwrap());
