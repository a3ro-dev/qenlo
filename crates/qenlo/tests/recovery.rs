//! Process-exit acceptance, not a simulation of power removal or a torn filesystem.

use qenlo::{Collection, CollectionConfig, Filter};
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[test]
fn process_exit_child() {
    let Some(path) = std::env::var_os("QENLO_RECOVERY_TEST_PATH") else {
        return;
    };
    let collection = block_on(Collection::create(&path, CollectionConfig::cpu_exact(2))).unwrap();
    collection.add(1, u64::MAX, i64::MIN, &[1.0, 0.0]).unwrap();
    if std::env::var_os("QENLO_RECOVERY_TEST_PENDING").is_some() {
        std::fs::write(
            std::path::Path::new(&path).join("canonical-00000000000000000002.pending"),
            b"interrupted-write",
        )
        .unwrap();
    }
    // Explicitly bypass Drop/close/flush to exercise OS lock release and durability.
    std::process::exit(0);
}

#[test]
fn committed_data_survives_process_exit_and_partial_staging() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    for pending in [false, true] {
        let path = std::env::temp_dir().join(format!(
            "qenlo-process-{}-{nonce}-{pending}",
            std::process::id()
        ));
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "process_exit_child", "--nocapture"])
            .env("QENLO_RECOVERY_TEST_PATH", &path)
            .env_remove("QENLO_RECOVERY_TEST_PENDING");
        if pending {
            command.env("QENLO_RECOVERY_TEST_PENDING", "1");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let collection = block_on(Collection::open(&path, CollectionConfig::cpu_exact(2))).unwrap();
        assert_eq!(collection.filter(&Filter::ALL), [1]);
        assert_eq!(collection.stats().recovered_interrupted_write, pending);
        collection.close().unwrap();
        std::fs::remove_dir_all(path).unwrap();
    }
}
