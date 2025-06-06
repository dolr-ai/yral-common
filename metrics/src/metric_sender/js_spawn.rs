use std::convert::Infallible;

use wasm_bindgen_futures::spawn_local;

use crate::metrics::{Metric, MetricEvent, MetricEventList};

use super::LocalMetricEventTx;

/// takes a metric sender and converts it into non-blocking concurrent metric sender
/// each future is spawned on the current thread to be run in the background
/// PS: if metric is not sent, the metric is ignored and logged instead
#[derive(Clone)]
pub struct JsSpawnMetricTx<Tx: LocalMetricEventTx + Clone + 'static>(pub Tx);

impl<Tx: LocalMetricEventTx + Clone + 'static> LocalMetricEventTx for JsSpawnMetricTx<Tx> {
    type Error = Infallible;

    async fn push_local<M: Metric + Send + 'static>(
        &self,
        ev: MetricEvent<M>,
    ) -> Result<(), Self::Error> {
        let tx = self.0.clone();
        spawn_local(async move {
            let res = tx.push_local(ev).await;
            if let Err(e) = res {
                log::warn!("failed to send metric {e}")
            }
        });
        Ok(())
    }

    async fn push_list_local<M: Metric + Send + 'static>(
        &self,
        ev: MetricEventList<M>,
    ) -> Result<(), Self::Error> {
        let tx = self.0.clone();
        spawn_local(async move {
            let res = tx.push_list_local(ev).await;
            if let Err(e) = res {
                log::warn!("failed to send metric {e}")
            }
        });
        Ok(())
    }
}
