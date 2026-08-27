//! Host-owned OTLP HTTP/protobuf setup.
//!
//! Run with an OTLP collector listening on the standard HTTP endpoint:
//! `cargo run -p qenlo-bench --features otlp --example otlp`.

use std::{error::Error, time::Duration};

use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_otlp::{MetricExporter, Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::{BatchConfigBuilder, BatchSpanProcessor, SdkTracerProvider},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let resource = Resource::builder().with_service_name("qenlo-bench").build();
    let span_exporter = SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_timeout(Duration::from_secs(5))
        .build()?;
    let span_processor = BatchSpanProcessor::builder(span_exporter)
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_max_queue_size(2_048)
                .with_max_export_batch_size(512)
                .build(),
        )
        .build();
    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(span_processor)
        .with_resource(resource.clone())
        .build();

    let metric_exporter = MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_timeout(Duration::from_secs(5))
        .build()?;
    let metric_reader = PeriodicReader::builder(metric_exporter)
        .with_interval(Duration::from_secs(10))
        .build();
    let meter_provider = SdkMeterProvider::builder()
        .with_reader(metric_reader)
        .with_resource(resource)
        .build();

    let tracer = tracer_provider.tracer("qenlo-host");
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();
    global::set_meter_provider(meter_provider.clone());

    let meter = global::meter("qenlo-host");
    let searches = meter.u64_counter("qenlo.search.operations").build();
    let _span = tracing::info_span!("qenlo.search", backend = "cpu", outcome = "ok").entered();
    searches.add(
        1,
        &[
            KeyValue::new("backend", "cpu"),
            KeyValue::new("operation", "search"),
            KeyValue::new("outcome", "example"),
        ],
    );

    // Exporter failures are isolated from search work; shutdown only happens at host exit.
    let _ = tracer_provider.shutdown();
    let _ = meter_provider.shutdown();
    Ok(())
}
