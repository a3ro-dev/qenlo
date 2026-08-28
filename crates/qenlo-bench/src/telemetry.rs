//! Host-owned, bounded OTLP HTTP/protobuf setup. No subscriber is installed here.

use opentelemetry_otlp::{MetricExporter, Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::{BatchConfigBuilder, BatchSpanProcessor, SdkTracerProvider},
};
use std::{error::Error, time::Duration};

/// Providers belong to the host, never to a collection.
pub struct HostTelemetry {
    pub traces: SdkTracerProvider,
    pub metrics: SdkMeterProvider,
}

impl HostTelemetry {
    /// Build exporters outside async execution (use `spawn_blocking` in Tokio).
    /// The blocking HTTP client matches the SDK's dedicated worker threads.
    /// `endpoint` overrides
    /// the collector base URL; `None` uses standard OTEL environment configuration.
    /// Requests time out; the trace queue holds at most 32 spans and drops overflow.
    pub fn new(
        endpoint: Option<&str>,
        timeout: Duration,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let resource = Resource::builder().with_service_name("qenlo-bench").build();
        let mut traces = SpanExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(timeout);
        let mut metrics = MetricExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_timeout(timeout);
        if let Some(endpoint) = endpoint {
            traces = traces.with_endpoint(format!("{}/v1/traces", endpoint.trim_end_matches('/')));
            metrics =
                metrics.with_endpoint(format!("{}/v1/metrics", endpoint.trim_end_matches('/')));
        }
        let processor = BatchSpanProcessor::builder(traces.build()?)
            .with_batch_config(
                BatchConfigBuilder::default()
                    .with_max_queue_size(32)
                    .with_max_export_batch_size(8)
                    .with_scheduled_delay(Duration::from_millis(100))
                    .build(),
            )
            .build();
        Ok(Self {
            traces: SdkTracerProvider::builder()
                .with_span_processor(processor)
                .with_resource(resource.clone())
                .build(),
            metrics: SdkMeterProvider::builder()
                .with_reader(
                    PeriodicReader::builder(metrics.build()?)
                        .with_interval(Duration::from_secs(10))
                        .build(),
                )
                .with_resource(resource)
                .build(),
        })
    }

    /// Shut down only after spans have ended, outside query timing. Export errors
    /// are returned to the host and never affect search results.
    /// SDK 0.32.1 ignores the meter provider's timeout argument; its periodic
    /// reader has a fixed five-second shutdown wait. HTTP requests still use the
    /// configured timeout. This host registers no potentially blocking callbacks.
    pub fn shutdown(self) -> (bool, bool) {
        let traces = self
            .traces
            .shutdown_with_timeout(Duration::from_secs(2))
            .is_ok();
        let metrics = self.metrics.shutdown().is_ok();
        (traces, metrics)
    }
}
