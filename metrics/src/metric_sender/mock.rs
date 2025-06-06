use std::{convert::Infallible, fmt::Debug};

use crate::metrics::{Metric, MetricEvent, MetricEventList};

use super::{LocalMetricEventTx, MetricEventTx};

#[derive(Default, Clone, Copy)]
pub struct MockMetricEventTx;

impl MockMetricEventTx {
    fn push_inner(&self, ev: impl Debug) {
        log::debug!("mock metric received: {ev:?}");
    }
}

impl MetricEventTx for MockMetricEventTx {
    type Error = Infallible;

    async fn push<M: Metric + Send + 'static>(
        &self,
        ev: MetricEvent<M>,
    ) -> Result<(), Self::Error> {
        self.push_inner(ev);

        Ok(())
    }

    async fn push_list<M: Metric + Send + 'static>(
        &self,
        ev: MetricEventList<M>,
    ) -> Result<(), Self::Error> {
        self.push_inner(ev);

        Ok(())
    }
}

#[derive(Clone)]
pub enum MaybeMockMetricEventTx<Tx> {
    Mock(MockMetricEventTx),
    Real(Tx),
}

impl<Tx> Default for MaybeMockMetricEventTx<Tx> {
    fn default() -> Self {
        Self::Mock(MockMetricEventTx)
    }
}

impl<Tx: MetricEventTx + Sync> MetricEventTx for MaybeMockMetricEventTx<Tx> {
    type Error = Tx::Error;

    async fn push<M: Metric + Send + 'static>(
        &self,
        ev: MetricEvent<M>,
    ) -> Result<(), Self::Error> {
        match self {
            Self::Mock(m) => {
                m.push(ev).await.unwrap();
                Ok(())
            }
            Self::Real(m) => m.push(ev).await,
        }
    }

    async fn push_list<M: Metric + Send + 'static>(
        &self,
        ev: MetricEventList<M>,
    ) -> Result<(), Self::Error> {
        match self {
            Self::Mock(m) => {
                m.push_list(ev).await.unwrap();
                Ok(())
            }
            Self::Real(m) => m.push_list(ev).await,
        }
    }
}

#[derive(Clone)]
pub enum MaybeMockLocalMetricEventTx<Tx> {
    Mock(MockMetricEventTx),
    Real(Tx),
}

impl<Tx> Default for MaybeMockLocalMetricEventTx<Tx> {
    fn default() -> Self {
        Self::Mock(MockMetricEventTx)
    }
}

impl<Tx: LocalMetricEventTx> LocalMetricEventTx for MaybeMockLocalMetricEventTx<Tx> {
    type Error = Tx::Error;

    async fn push_local<M: Metric + Send + 'static>(
        &self,
        ev: MetricEvent<M>,
    ) -> Result<(), Self::Error> {
        match self {
            Self::Mock(m) => {
                m.push(ev).await.unwrap();
                Ok(())
            }
            Self::Real(m) => m.push_local(ev).await,
        }
    }

    async fn push_list_local<M: Metric + Send + 'static>(
        &self,
        ev: MetricEventList<M>,
    ) -> Result<(), Self::Error> {
        match self {
            Self::Mock(m) => {
                m.push_list(ev).await.unwrap();
                Ok(())
            }
            Self::Real(m) => m.push_list_local(ev).await,
        }
    }
}
