package qenlo

import (
	"errors"
	"path/filepath"
	"testing"
)

func fixture() []Record {
	return []Record{
		{ID: 9, UserID: 7, Timestamp: -5, Vector: []float32{1, 0, 0}},
		{ID: 2, UserID: 7, Timestamp: 0, Vector: []float32{2, 0, 0}},
		{ID: 4, UserID: 8, Timestamp: 10, Vector: []float32{0, 1, 0}},
		{ID: 6, UserID: 7, Timestamp: 20, Vector: []float32{0, 0, 1}},
	}
}

func TestTypedFilterOrderingAndTelemetry(t *testing.T) {
	db, err := New(3); if err != nil { t.Fatal(err) }; defer db.Close()
	if err := db.AddBatch(fixture()); err != nil { t.Fatal(err) }
	response, err := db.Search([]float32{1, 0, 0}, Filter{UserID: Uint64(7), TimestampLower: Int64(-5), TimestampUpper: Int64(20)}, 10)
	if err != nil { t.Fatal(err) }
	if len(response.Results) != 2 || response.Results[0].ID != 2 || response.Results[1].ID != 9 { t.Fatalf("unexpected results: %#v", response.Results) }
	if response.Report.ActualBackend != "Cpu" || response.Report.Algorithm != "Exact" || response.Report.OperationID == 0 { t.Fatalf("unexpected report: %#v", response.Report) }
}

func TestAtomicBatchAndNonReusableIDs(t *testing.T) {
	db, _ := New(3); defer db.Close()
	rows := fixture(); if err := db.Add(rows[0]); err != nil { t.Fatal(err) }
	if err := db.AddBatch([]Record{rows[1], rows[0]}); err == nil { t.Fatal("expected atomic batch failure") }
	stats, _ := db.Stats(); if stats.Rows != 1 { t.Fatalf("partial batch committed: %#v", stats) }
	if err := db.Delete(9); err != nil { t.Fatal(err) }
	if err := db.Add(rows[0]); err == nil { t.Fatal("deleted ID was reused") }
}

func TestDurableReopen(t *testing.T) {
	path := filepath.Join(t.TempDir(), "vectors.qenlo")
	db, err := Create(path, 3); if err != nil { t.Fatal(err) }
	if err := db.AddBatch(fixture()); err != nil { t.Fatal(err) }
	if err := db.DeleteBatch([]uint64{2, 4}); err != nil { t.Fatal(err) }
	if err := db.Flush(); err != nil { t.Fatal(err) }
	if err := db.Close(); err != nil { t.Fatal(err) }
	db, err = Open(path, 3); if err != nil { t.Fatal(err) }; defer db.Close()
	stats, _ := db.Stats(); if stats.LiveRows != 2 { t.Fatalf("unexpected stats: %#v", stats) }
}

func TestValidationAndLifecycle(t *testing.T) {
	db, _ := New(3)
	if err := db.Add(Record{ID: 1, UserID: 1, Vector: []float32{1}}); err == nil { t.Fatal("expected dimension error") }
	if _, err := db.Search([]float32{1, 0, 0}, Filter{}, 0); err == nil { t.Fatal("expected k error") }
	if err := db.Add(Record{ID: 1, UserID: 1, Vector: []float32{0, 0, 0}}); err == nil { t.Fatal("expected native vector error") }
	if err := db.Close(); err != nil { t.Fatal(err) }; if err := db.Close(); err != nil { t.Fatal(err) }
	_, err := db.Stats(); var native *Error; if !errors.As(err, &native) { t.Fatalf("expected typed closed error: %v", err) }
}
