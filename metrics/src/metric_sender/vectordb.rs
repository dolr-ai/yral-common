use reqwest::Url;

use crate::metrics::{Metric, MetricEvent, MetricEventList};

const VECTOR_DB_URL: &str = "https://vector-dev-yral.fly.dev/";

/// Sends metrics to Yral's vectordb instance
#[derive(Clone)]
pub struct VectorDbMetricTx {
    client: reqwest::Client,
    ingest_url: Url,
}

impl Default for VectorDbMetricTx {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
            ingest_url: VECTOR_DB_URL.parse().unwrap(),
        }
    }
}

impl VectorDbMetricTx {
    async fn push_inner<M: Metric + Send>(&self, ev: MetricEvent<M>) -> Result<(), reqwest::Error> {
        _ = self
            .client
            .post(self.ingest_url.clone())
            .json(&ev)
            .send()
            .await?;
        Ok(())
    }

    async fn push_list_inner<M: Metric + Send>(
        &self,
        ev: MetricEventList<M>,
    ) -> Result<(), reqwest::Error> {
        _ = self
            .client
            .post(self.ingest_url.clone())
            .json(&ev)
            .send()
            .await?;
        Ok(())
    }
}

#[cfg(feature = "js")]
impl super::LocalMetricEventTx for VectorDbMetricTx {
    type Error = reqwest::Error;

    async fn push_local<M: Metric + Send>(&self, ev: MetricEvent<M>) -> Result<(), Self::Error> {
        self.push_inner(ev).await
    }

    async fn push_list_local<M: Metric + Send>(
        &self,
        ev: MetricEventList<M>,
    ) -> Result<(), Self::Error> {
        self.push_list_inner(ev).await
    }
}

#[cfg(not(feature = "js"))]
impl super::MetricEventTx for VectorDbMetricTx {
    type Error = reqwest::Error;

    async fn push<M: Metric + Send>(&self, ev: MetricEvent<M>) -> Result<(), Self::Error> {
        self.push_inner(ev).await
    }

    async fn push_list<M: Metric + Send>(&self, ev: MetricEventList<M>) -> Result<(), Self::Error> {
        self.push_list_inner(ev).await
    }
}
