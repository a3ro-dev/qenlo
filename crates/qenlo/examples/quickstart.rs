//! Run with a NEW directory (existing data is never replaced):
//! cargo run -p qenlo --example quickstart -- ./demo.qenlo cpu
//! cargo run -p qenlo --features gpu-wgpu --example quickstart -- ./gpu-demo.qenlo gpu
//! cargo run -p qenlo --features usearch --example quickstart -- ./ann-demo.qenlo usearch
//!
//! These hand-written vectors demonstrate the API, not embedding quality or speed.

use qenlo::{Collection, CollectionConfig, Filter, NewRecord, TimestampRange};
use std::{
    error::Error,
    future::Future,
    path::PathBuf,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

fn main() -> Result<(), Box<dyn Error>> {
    block_on(run())
}

async fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = PathBuf::from(
        args.next()
            .ok_or("usage: quickstart NEW_DIRECTORY [cpu|usearch|gpu|automatic]")?,
    );
    let backend = args.next().unwrap_or_else(|| "cpu".into());
    if args.next().is_some() {
        return Err("usage: quickstart NEW_DIRECTORY [cpu|usearch|gpu|automatic]".into());
    }
    let mut config = CollectionConfig::cpu_exact(3);
    config.backend = match backend.to_str() {
        Some("cpu") => qenlo::BackendSelection::CpuExact,
        #[cfg(feature = "usearch")]
        Some("usearch") => qenlo::BackendSelection::Usearch,
        #[cfg(feature = "gpu-wgpu")]
        Some("gpu") => qenlo::BackendSelection::WgpuRequired(qenlo::GpuFilterMode::GpuPredicate),
        #[cfg(feature = "gpu-wgpu")]
        Some("automatic") => qenlo::BackendSelection::Automatic(qenlo::GpuFilterMode::GpuPredicate),
        _ => {
            return Err(
                "unknown or disabled backend; enable usearch or gpu-wgpu when building".into(),
            );
        }
    };
    let collection = Collection::create(&path, config.clone()).await?;
    #[cfg(feature = "gpu-wgpu")]
    if let Some(adapter) = collection.gpu_capabilities() {
        println!("adapter={} API={}", adapter.adapter_name, adapter.backend);
    }
    collection.add_batch(&[
        NewRecord {
            id: 1,
            user_id: 7,
            timestamp: 100,
            vector: vec![1.0, 0.0, 0.0],
        },
        NewRecord {
            id: 2,
            user_id: 7,
            timestamp: 200,
            vector: vec![0.8, 0.2, 0.0],
        },
        NewRecord {
            id: 3,
            user_id: 8,
            timestamp: 100,
            vector: vec![1.0, 0.0, 0.0],
        },
    ])?;
    // AND: tenant 7, timestamp >= 100 and < 300. Row 3 must not leak across tenants.
    let filter = Filter::new(Some(7), TimestampRange::new(Some(100), Some(300)));
    collection.prepare().await?;
    let found = collection.search(&[1.0, 0.0, 0.0], &filter, 10).await?;
    assert_eq!(
        found.results.iter().map(|hit| hit.id).collect::<Vec<_>>(),
        [1, 2]
    );
    println!("filtered results: {:?}", found.results);
    println!(
        "requested={:?} actual={:?} generation={} query={:?} fallback={:?}",
        found.report.requested_backend,
        found.report.actual_backend,
        found.report.index_generation,
        found.report.total_duration,
        found.report.fallback_reason,
    );
    println!(
        "dispatches={:?} readback={:?}",
        found.report.dispatch_count, found.report.readback_bytes
    );

    collection.delete(1)?;
    let remaining = collection.search(&[1.0, 0.0, 0.0], &filter, 10).await?;
    assert_eq!(
        remaining
            .results
            .iter()
            .map(|hit| hit.id)
            .collect::<Vec<_>>(),
        [2]
    );
    assert!(remaining.report.rebuilt);
    collection.close()?;
    drop(collection);

    let reopened = Collection::open(&path, config).await?;
    let persisted = reopened.search(&[1.0, 0.0, 0.0], &filter, 10).await?;
    assert_eq!(persisted.results, remaining.results);
    println!("after delete + reopen: {:?}", persisted.results);
    println!(
        "durable generation={:?}; data kept at {}",
        reopened.stats().durable_generation,
        path.display()
    );
    reopened.close()?;
    Ok(())
}

// A tiny native blocking executor; applications may use their existing executor.
// The waker unparks this thread, including notifications before park() is called.
fn block_on<F: Future>(future: F) -> F::Output {
    struct ThreadWake(std::thread::Thread);
    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}
