use qenlo::{Filter, TimestampRange};
use qenlo_browser::state::BrowserSession;

#[tokio::test]
async fn test_browser_session_lifecycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let col_path = temp_dir.path().join("test_col.qenlo");

    let mut session = BrowserSession::new();

    // Create collection with 3 dimensions
    let stats = session
        .create_collection(&col_path, 3)
        .await
        .expect("create collection");
    assert_eq!(stats.dimension, 3);
    assert_eq!(stats.rows, 0);

    // Add 3 records
    session
        .add_record(1, 10, 100, &[1.0, 0.0, 0.0])
        .expect("insert 1");
    session
        .add_record(2, 10, 200, &[0.0, 1.0, 0.0])
        .expect("insert 2");
    session
        .add_record(3, 20, 300, &[0.0, 0.0, 1.0])
        .expect("insert 3");

    let status = session.get_status();
    assert_eq!(status.live_rows, 3);
    assert_eq!(status.dimension, 3);

    // Scan records
    let paginated = session.scan_records(0, 10, None).expect("scan records");
    assert_eq!(paginated.total, 3);
    assert_eq!(paginated.records.len(), 3);

    // Search with query vector [1.0, 0.0, 0.0]
    let search_res = session
        .search(&[1.0, 0.0, 0.0], &Filter::ALL, 5)
        .await
        .expect("search");
    assert!(!search_res.results.is_empty());
    assert_eq!(search_res.results[0].id, 1);
    assert!((search_res.results[0].distance - 0.0).abs() < 1e-4);

    // Search with filter (User ID = 20)
    let filtered_search = session
        .search(
            &[1.0, 0.0, 0.0],
            &Filter::new(Some(20), TimestampRange::ALL),
            5,
        )
        .await
        .expect("filtered search");
    assert_eq!(filtered_search.results.len(), 1);
    assert_eq!(filtered_search.results[0].id, 3);

    // Delete record #1
    session.delete_record(1).expect("delete record 1");
    let after_del = session.get_status();
    assert_eq!(after_del.live_rows, 2);
    assert_eq!(after_del.tombstones, 1);

    // Flush & compact
    session.flush().expect("flush collection");

    // Storage inspector details
    let storage = session.get_storage_details();
    assert!(!storage.files.is_empty());
    assert!(
        storage
            .files
            .iter()
            .any(|f| f.name.ends_with(".qdb") || f.name == "HEAD")
    );
}
