//! Host-owned OTLP HTTP/protobuf, using the standard OTEL environment variables.
//! `cargo run -p qenlo-bench --features otlp --example otlp`

use opentelemetry::{KeyValue, metrics::MeterProvider as _, trace::TracerProvider as _};
use qenlo::{Collection, CollectionConfig, Filter};
use qenlo_bench::telemetry::HostTelemetry;
use std::{error::Error, time::Duration};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::layer::SubscriberExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let telemetry =
        tokio::task::spawn_blocking(|| HostTelemetry::new(None, Duration::from_millis(500)))
            .await??;
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(telemetry.traces.tracer("qenlo-host")));
    let searches = telemetry
        .metrics
        .meter("qenlo-host")
        .u64_counter("qenlo.search.operations")
        .build();
    // The scoped subscriber follows this future; no process-global subscriber.
    let result = async {
        let collection = Collection::new(CollectionConfig::cpu_exact(2)).await?;
        collection.add(1, 7, 10, &[1.0, 0.0])?;
        let response = collection.search(&[1.0, 0.0], &Filter::ALL, 1).await?;
        searches.add(
            1,
            &[
                KeyValue::new("backend", "cpu"),
                KeyValue::new("operation", "search"),
                KeyValue::new("outcome", "ok"),
            ],
        );
        assert_eq!(response.results[0].id, 1);
        Ok::<_, qenlo::Error>(())
    }
    .with_subscriber(subscriber)
    .await;
    // All query spans have closed before shutdown. Export failures are separate
    // from the query outcome; never print exporter errors containing endpoints.
    let (traces_ok, metrics_ok) = tokio::task::spawn_blocking(|| telemetry.shutdown()).await?;
    eprintln!("telemetry shutdown: traces_ok={traces_ok} metrics_ok={metrics_ok}");
    result?;
    Ok(())
}
