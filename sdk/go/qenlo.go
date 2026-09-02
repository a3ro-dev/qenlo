// Package qenlo provides type-safe Go bindings for the embedded Qenlo database.
package qenlo

/*
#cgo CFLAGS: -I${SRCDIR}/native
#cgo windows,amd64 LDFLAGS: -L${SRCDIR}/native/windows-amd64 -lqenlo_ffi
#cgo linux,amd64 LDFLAGS: -L${SRCDIR}/native/linux-amd64 -lqenlo_ffi -Wl,-rpath,${SRCDIR}/native/linux-amd64
#cgo linux,arm64 LDFLAGS: -L${SRCDIR}/native/linux-arm64 -lqenlo_ffi -Wl,-rpath,${SRCDIR}/native/linux-arm64
#cgo darwin,amd64 LDFLAGS: -L${SRCDIR}/native/darwin-amd64 -lqenlo_ffi
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/native/darwin-arm64 -lqenlo_ffi
#include "qenlo.h"
#include <stdlib.h>
*/
import "C"

import (
	"encoding/json"
	"errors"
	"fmt"
	"runtime"
	"strconv"
	"sync"
	"unsafe"
)

// Error is a validation, lifecycle, storage, or native execution failure.
type Error struct{ Message string }

func (e *Error) Error() string { return e.Message }

// Record is one canonical vector and its filterable metadata.
type Record struct {
	ID        uint64
	UserID    uint64
	Timestamp int64
	Vector    []float32
}

// Filter combines optional user equality with lower-inclusive, upper-exclusive timestamps.
type Filter struct {
	UserID         *uint64
	TimestampLower *int64
	TimestampUpper *int64
}

// Uint64 returns a pointer suitable for an optional filter field.
func Uint64(value uint64) *uint64 { return &value }

// Int64 returns a pointer suitable for an optional filter field.
func Int64(value int64) *int64 { return &value }

// SearchResult is ordered by cosine distance, then ID.
type SearchResult struct {
	ID       uint64
	Distance float32
}

// ExecutionReport explains the work that completed the search.
type ExecutionReport struct {
	OperationID      uint64
	RequestedBackend string
	ActualBackend    string
	Algorithm        string
	FilterExecution  string
	IndexGeneration  uint64
	Rebuilt          bool
	RoutingReason    *string
	FallbackReason   *string
	TotalDurationNS  uint64
	LockWaitNS       uint64
	EligibleRows     *uint64
	UploadBytes      *uint64
	ReadbackBytes    *uint64
	AllocationBytes  *uint64
	DispatchCount    *uint32
	Candidates       *uint64
	BatchSize        int
}

// SearchResponse contains ordered hits and telemetry from one committed generation.
type SearchResponse struct {
	Results []SearchResult
	Report  ExecutionReport
}

// CollectionStats describes canonical, durable, and lifecycle state.
type CollectionStats struct {
	Dimension                 int
	Rows                      int
	LiveRows                  int
	Generation                uint64
	PreparedGeneration        *uint64
	DurableGeneration         *uint64
	RecoveredInterruptedWrite bool
	Closed                    bool
}

// Collection owns one native Qenlo handle and is safe for concurrent searches.
type Collection struct {
	mu        sync.RWMutex
	handle    *C.QenloCollection
	dimension int
}

func newCollection(handle *C.QenloCollection, dimension int) (*Collection, error) {
	if handle == nil {
		return nil, nativeError()
	}
	db := &Collection{handle: handle, dimension: dimension}
	runtime.SetFinalizer(db, func(value *Collection) { _ = value.Close() })
	return db, nil
}

func validateDimension(dimension int) error {
	if dimension <= 0 {
		return errors.New("dimension must be positive")
	}
	return nil
}

// New creates an in-memory exact-CPU collection.
func New(dimension int) (*Collection, error) {
	if err := validateDimension(dimension); err != nil {
		return nil, err
	}
	return newCollection(C.qenlo_collection_new(C.size_t(dimension)), dimension)
}

// Create creates durable state in a new or empty directory.
func Create(path string, dimension int) (*Collection, error) {
	if err := validateDimension(dimension); err != nil {
		return nil, err
	}
	value := C.CString(path)
	defer C.free(unsafe.Pointer(value))
	return newCollection(C.qenlo_collection_create(value, C.size_t(dimension)), dimension)
}

// Open recovers durable state under an exclusive process lock.
func Open(path string, dimension int) (*Collection, error) {
	if err := validateDimension(dimension); err != nil {
		return nil, err
	}
	value := C.CString(path)
	defer C.free(unsafe.Pointer(value))
	return newCollection(C.qenlo_collection_open(value, C.size_t(dimension)), dimension)
}

// ImportQN imports a checksummed .qn snapshot into a mutable in-memory collection.
func ImportQN(path string, dimension int) (*Collection, error) {
	if err := validateDimension(dimension); err != nil {
		return nil, err
	}
	value := C.CString(path)
	defer C.free(unsafe.Pointer(value))
	return newCollection(C.qenlo_collection_import_qn(value, C.size_t(dimension)), dimension)
}

func nativeError() error {
	value := C.qenlo_last_error()
	if value == nil {
		return &Error{Message: "unknown Qenlo native error"}
	}
	defer C.qenlo_string_free(value)
	return &Error{Message: C.GoString(value)}
}

func check(status C.int32_t) error {
	if status != 0 {
		return nativeError()
	}
	return nil
}

func takeString(value *C.char) (string, error) {
	if value == nil {
		return "", nativeError()
	}
	defer C.qenlo_string_free(value)
	return C.GoString(value), nil
}

func (db *Collection) withHandle(operation func(*C.QenloCollection) error) error {
	db.mu.RLock()
	defer db.mu.RUnlock()
	if db.handle == nil {
		return &Error{Message: "collection is closed"}
	}
	return operation(db.handle)
}

func (db *Collection) validateVector(vector []float32) error {
	if len(vector) != db.dimension {
		return fmt.Errorf("expected vector dimension %d, got %d", db.dimension, len(vector))
	}
	return nil
}

// Add validates, normalizes, and atomically adds one record.
func (db *Collection) Add(record Record) error {
	if err := db.validateVector(record.Vector); err != nil {
		return err
	}
	return db.withHandle(func(handle *C.QenloCollection) error {
		return check(C.qenlo_add(handle, C.uint64_t(record.ID), C.uint64_t(record.UserID), C.int64_t(record.Timestamp), (*C.float)(unsafe.Pointer(&record.Vector[0])), C.size_t(len(record.Vector))))
	})
}

// AddBatch commits every record as one ordered atomic transaction.
func (db *Collection) AddBatch(records []Record) error {
	if len(records) == 0 {
		return nil
	}
	ids := make([]uint64, len(records))
	users := make([]uint64, len(records))
	timestamps := make([]int64, len(records))
	vectors := make([]float32, len(records)*db.dimension)
	for row, record := range records {
		if err := db.validateVector(record.Vector); err != nil {
			return err
		}
		ids[row], users[row], timestamps[row] = record.ID, record.UserID, record.Timestamp
		copy(vectors[row*db.dimension:(row+1)*db.dimension], record.Vector)
	}
	return db.withHandle(func(handle *C.QenloCollection) error {
		return check(C.qenlo_add_batch(handle, (*C.uint64_t)(unsafe.Pointer(&ids[0])), (*C.uint64_t)(unsafe.Pointer(&users[0])), (*C.int64_t)(unsafe.Pointer(&timestamps[0])), (*C.float)(unsafe.Pointer(&vectors[0])), C.size_t(len(records)), C.size_t(db.dimension)))
	})
}

// Delete removes one live record. IDs are never reusable.
func (db *Collection) Delete(id uint64) error {
	return db.withHandle(func(handle *C.QenloCollection) error {
		return check(C.qenlo_delete(handle, C.uint64_t(id)))
	})
}

// DeleteBatch deletes every ID in one ordered atomic transaction.
func (db *Collection) DeleteBatch(ids []uint64) error {
	if len(ids) == 0 {
		return nil
	}
	return db.withHandle(func(handle *C.QenloCollection) error {
		return check(C.qenlo_delete_batch(handle, (*C.uint64_t)(unsafe.Pointer(&ids[0])), C.size_t(len(ids))))
	})
}

type wireSearch struct {
	Results []struct {
		ID       string  `json:"id"`
		Distance float32 `json:"distance"`
	} `json:"results"`
	Report struct {
		OperationID      string  `json:"operation_id"`
		RequestedBackend string  `json:"requested_backend"`
		ActualBackend    string  `json:"actual_backend"`
		Algorithm        string  `json:"algorithm"`
		FilterExecution  string  `json:"filter_execution"`
		IndexGeneration  string  `json:"index_generation"`
		Rebuilt          bool    `json:"rebuilt"`
		RoutingReason    *string `json:"routing_reason"`
		FallbackReason   *string `json:"fallback_reason"`
		TotalDurationNS  string  `json:"total_duration_ns"`
		LockWaitNS       string  `json:"lock_wait_ns"`
		EligibleRows     *string `json:"eligible_rows"`
		UploadBytes      *string `json:"upload_bytes"`
		ReadbackBytes    *string `json:"readback_bytes"`
		AllocationBytes  *string `json:"allocation_bytes"`
		DispatchCount    *uint32 `json:"dispatch_count"`
		Candidates       *string `json:"candidates"`
		BatchSize        int     `json:"batch_size"`
	} `json:"report"`
}

func parseUint(value string) (uint64, error) { return strconv.ParseUint(value, 10, 64) }

func parseOptional(value *string) (*uint64, error) {
	if value == nil {
		return nil, nil
	}
	parsed, err := parseUint(*value)
	return &parsed, err
}

// Search returns distance-then-ID ordered hits and an execution report.
func (db *Collection) Search(query []float32, filter Filter, k int) (SearchResponse, error) {
	if err := db.validateVector(query); err != nil {
		return SearchResponse{}, err
	}
	if k < 1 || k > 64 {
		return SearchResponse{}, errors.New("k must be in 1..=64")
	}
	var raw string
	err := db.withHandle(func(handle *C.QenloCollection) error {
		var user uint64
		var lower, upper int64
		if filter.UserID != nil {
			user = *filter.UserID
		}
		if filter.TimestampLower != nil {
			lower = *filter.TimestampLower
		}
		if filter.TimestampUpper != nil {
			upper = *filter.TimestampUpper
		}
		value, err := takeString(C.qenlo_search(handle, (*C.float)(unsafe.Pointer(&query[0])), C.size_t(len(query)), C.bool(filter.UserID != nil), C.uint64_t(user), C.bool(filter.TimestampLower != nil), C.int64_t(lower), C.bool(filter.TimestampUpper != nil), C.int64_t(upper), C.size_t(k)))
		raw = value
		return err
	})
	if err != nil {
		return SearchResponse{}, err
	}
	var wire wireSearch
	if err := json.Unmarshal([]byte(raw), &wire); err != nil {
		return SearchResponse{}, err
	}
	response := SearchResponse{Results: make([]SearchResult, len(wire.Results))}
	for index, hit := range wire.Results {
		id, err := parseUint(hit.ID)
		if err != nil {
			return SearchResponse{}, err
		}
		response.Results[index] = SearchResult{ID: id, Distance: hit.Distance}
	}
	operationID, err := parseUint(wire.Report.OperationID)
	if err != nil {
		return SearchResponse{}, err
	}
	indexGeneration, err := parseUint(wire.Report.IndexGeneration)
	if err != nil {
		return SearchResponse{}, err
	}
	total, err := parseUint(wire.Report.TotalDurationNS)
	if err != nil {
		return SearchResponse{}, err
	}
	lock, err := parseUint(wire.Report.LockWaitNS)
	if err != nil {
		return SearchResponse{}, err
	}
	eligible, err := parseOptional(wire.Report.EligibleRows)
	if err != nil {
		return SearchResponse{}, err
	}
	upload, err := parseOptional(wire.Report.UploadBytes)
	if err != nil {
		return SearchResponse{}, err
	}
	readback, err := parseOptional(wire.Report.ReadbackBytes)
	if err != nil {
		return SearchResponse{}, err
	}
	allocation, err := parseOptional(wire.Report.AllocationBytes)
	if err != nil {
		return SearchResponse{}, err
	}
	candidates, err := parseOptional(wire.Report.Candidates)
	if err != nil {
		return SearchResponse{}, err
	}
	response.Report = ExecutionReport{OperationID: operationID, RequestedBackend: wire.Report.RequestedBackend, ActualBackend: wire.Report.ActualBackend, Algorithm: wire.Report.Algorithm, FilterExecution: wire.Report.FilterExecution, IndexGeneration: indexGeneration, Rebuilt: wire.Report.Rebuilt, RoutingReason: wire.Report.RoutingReason, FallbackReason: wire.Report.FallbackReason, TotalDurationNS: total, LockWaitNS: lock, EligibleRows: eligible, UploadBytes: upload, ReadbackBytes: readback, AllocationBytes: allocation, DispatchCount: wire.Report.DispatchCount, Candidates: candidates, BatchSize: wire.Report.BatchSize}
	return response, nil
}

// Stats returns canonical, durable, and lifecycle state without row payloads.
func (db *Collection) Stats() (CollectionStats, error) {
	var raw string
	err := db.withHandle(func(handle *C.QenloCollection) error {
		value, err := takeString(C.qenlo_stats(handle))
		raw = value
		return err
	})
	if err != nil {
		return CollectionStats{}, err
	}
	var wire struct {
		Dimension                 int     `json:"dimension"`
		Rows                      int     `json:"rows"`
		LiveRows                  int     `json:"live_rows"`
		Generation                string  `json:"generation"`
		PreparedGeneration        *string `json:"prepared_generation"`
		DurableGeneration         *string `json:"durable_generation"`
		RecoveredInterruptedWrite bool    `json:"recovered_interrupted_write"`
		Closed                    bool    `json:"closed"`
	}
	if err := json.Unmarshal([]byte(raw), &wire); err != nil {
		return CollectionStats{}, err
	}
	generation, err := parseUint(wire.Generation)
	if err != nil {
		return CollectionStats{}, err
	}
	prepared, err := parseOptional(wire.PreparedGeneration)
	if err != nil {
		return CollectionStats{}, err
	}
	durable, err := parseOptional(wire.DurableGeneration)
	if err != nil {
		return CollectionStats{}, err
	}
	return CollectionStats{Dimension: wire.Dimension, Rows: wire.Rows, LiveRows: wire.LiveRows, Generation: generation, PreparedGeneration: prepared, DurableGeneration: durable, RecoveredInterruptedWrite: wire.RecoveredInterruptedWrite, Closed: wire.Closed}, nil
}

// Flush compacts durable WAL state into a canonical snapshot.
func (db *Collection) Flush() error {
	return db.withHandle(func(handle *C.QenloCollection) error { return check(C.qenlo_flush(handle)) })
}

// ExportQN atomically exports the current generation to a new portable .qn file.
func (db *Collection) ExportQN(path string) error {
	value := C.CString(path)
	defer C.free(unsafe.Pointer(value))
	return db.withHandle(func(handle *C.QenloCollection) error {
		return check(C.qenlo_export_qn(handle, value))
	})
}

// Close releases the native collection. It is idempotent.
func (db *Collection) Close() error {
	db.mu.Lock()
	defer db.mu.Unlock()
	if db.handle == nil {
		return nil
	}
	handle := db.handle
	db.handle = nil
	runtime.SetFinalizer(db, nil)
	status := C.qenlo_close(handle)
	C.qenlo_collection_free(handle)
	return check(status)
}
