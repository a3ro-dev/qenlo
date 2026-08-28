#![cfg(feature = "otlp")]

use opentelemetry::{metrics::MeterProvider as _, trace::TracerProvider as _};
use qenlo::{Collection, CollectionConfig, Filter, TimestampRange};
use qenlo_bench::telemetry::HostTelemetry;
use std::{
    io::{self, Write},
    net::TcpListener,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt};

#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stalled_collector_preserves_results_privacy_and_bounded_shutdown() {
    // A real HTTP transport failure: accept connections but never return headers.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let (stop, stopped) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let mut connections = Vec::new();
        loop {
            if let Ok((socket, _)) = listener.accept() {
                connections.push(socket);
            }
            if stopped.recv_timeout(Duration::from_millis(5)).is_ok() {
                return connections.len();
            }
        }
    });
    let telemetry = tokio::task::spawn_blocking(move || {
        HostTelemetry::new(
            Some(&format!("http://{address}")),
            Duration::from_millis(100),
        )
    })
    .await
    .unwrap()
    .unwrap();
    let capture = Capture(Arc::new(Mutex::new(Vec::new())));
    let writer = capture.clone();
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .without_time()
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_writer(move || writer.clone()),
        )
        .with(tracing_opentelemetry::layer().with_tracer(telemetry.traces.tracer("privacy-test")));
    let collection = Collection::new(CollectionConfig::cpu_exact(2))
        .await
        .unwrap();
    collection
        .add(
            123,
            987654321012345,
            -876543210123456,
            &[0.12345679, 0.9876543],
        )
        .unwrap();
    let filter = Filter {
        user_id: Some(987654321012345),
        timestamp: TimestampRange::new(Some(-876543210123457), None),
    };
    let baseline = collection
        .search(&[0.12345679, 0.9876543], &filter, 1)
        .await
        .unwrap()
        .results;
    let started = Instant::now();
    async {
        collection
            .add(
                124,
                987654321012345,
                -876543210123456,
                &[0.12345679, 0.9876543],
            )
            .unwrap();
        collection.delete(124).unwrap();
        for diagnostics in [
            qenlo::Diagnostics::Basic,
            qenlo::Diagnostics::Detailed,
            qenlo::Diagnostics::Disabled,
        ] {
            collection.set_diagnostics(diagnostics);
            let before = capture.0.lock().unwrap().len();
            for _ in 0..80 {
                assert_eq!(
                    collection
                        .search(&[0.12345679, 0.9876543], &filter, 1)
                        .await
                        .unwrap()
                        .results,
                    baseline
                );
            }
            if diagnostics == qenlo::Diagnostics::Disabled {
                assert_eq!(
                    before,
                    capture.0.lock().unwrap().len(),
                    "disabled diagnostics must emit no spans"
                );
            }
        }
    }
    .with_subscriber(subscriber)
    .await;
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "export must not block query calls"
    );
    telemetry
        .metrics
        .meter("test")
        .u64_counter("qenlo.search.operations")
        .build()
        .add(80, &[]);
    let shutdown_started = Instant::now();
    let _ = tokio::task::spawn_blocking(|| telemetry.shutdown())
        .await
        .unwrap();
    let elapsed = shutdown_started.elapsed();
    stop.send(()).unwrap();
    assert!(
        server.join().unwrap() > 0,
        "test must exercise real HTTP export"
    );
    assert!(
        elapsed < Duration::from_secs(8),
        "trace 2s + SDK metrics 5s + scheduling margin"
    );
    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("qenlo.operation") && logs.contains("search"));
    for forbidden in [
        "987654321012345",
        "876543210123456",
        "876543210123457",
        "0.12345679",
        "0.9876543",
        "user_id",
        "timestamp",
        "vector",
    ] {
        assert!(
            !logs.contains(forbidden),
            "private field leaked: {forbidden}"
        );
    }
}
