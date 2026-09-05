"""Type-safe Python API for Qenlo. Importing it starts no threads or network work."""

from __future__ import annotations

import ctypes
import json
from dataclasses import dataclass
from os import PathLike
from typing import Any, Iterable, Literal, Sequence

from ._native import LIB, Handle, last_error, take_string

__all__ = [
    "Collection",
    "CollectionStats",
    "ExecutionMode",
    "ExecutionReport",
    "Filter",
    "GpuFilterMode",
    "QenloError",
    "Record",
    "SearchResponse",
    "SearchResult",
]

ExecutionMode = Literal["cpu", "automatic", "gpu-required"]
GpuFilterMode = Literal["cpu-mask", "cpu-rows", "gpu-predicate"]
_BACKENDS: dict[str, int] = {"cpu": 0, "automatic": 1, "gpu-required": 2}
_GPU_FILTERS: dict[str, int] = {"cpu-mask": 0, "cpu-rows": 1, "gpu-predicate": 2}
_DEFAULT_GPU_BUDGET_BYTES = 512 * 1024 * 1024


class QenloError(RuntimeError):
    """A validation, lifecycle, storage, or native execution failure."""


@dataclass(frozen=True, slots=True)
class Record:
    """One canonical vector and its filterable metadata."""

    id: int
    user_id: int
    timestamp: int
    vector: Sequence[float]


@dataclass(frozen=True, slots=True)
class Filter:
    """Optional user equality and lower-inclusive, upper-exclusive timestamp bounds."""

    user_id: int | None = None
    timestamp_lower: int | None = None
    timestamp_upper: int | None = None


@dataclass(frozen=True, slots=True)
class SearchResult:
    id: int
    distance: float


@dataclass(frozen=True, slots=True)
class ExecutionReport:
    operation_id: int
    requested_backend: str
    actual_backend: str
    algorithm: str
    filter_execution: str
    index_generation: int
    rebuilt: bool
    routing_reason: str | None
    fallback_reason: str | None
    total_duration_ns: int
    lock_wait_ns: int
    eligible_rows: int | None
    upload_bytes: int | None
    readback_bytes: int | None
    allocation_bytes: int | None
    dispatch_count: int | None
    candidates: int | None
    batch_size: int


@dataclass(frozen=True, slots=True)
class SearchResponse:
    results: tuple[SearchResult, ...]
    report: ExecutionReport


@dataclass(frozen=True, slots=True)
class CollectionStats:
    dimension: int
    rows: int
    live_rows: int
    generation: int
    prepared_generation: int | None
    durable_generation: int | None
    recovered_interrupted_write: bool
    closed: bool


@dataclass(frozen=True, slots=True)
class _CanonicalBuffer:
    generation: int
    dimension: int
    rows: int
    ids: ctypes.Array
    vectors: ctypes.Array


def _validate_u64(value: int, name: str) -> None:
    if not 0 <= value <= 2**64 - 1:
        raise ValueError(f"{name} must fit unsigned 64-bit integer")


def _validate_i64(value: int, name: str) -> None:
    if not -(2**63) <= value <= 2**63 - 1:
        raise ValueError(f"{name} must fit signed 64-bit integer")


class Collection:
    """An owned Qenlo collection. Close it explicitly or use a context manager."""

    def __init__(self, handle: Handle, dimension: int) -> None:
        if not handle:
            raise QenloError(last_error())
        self._handle: Handle | None = handle
        self.dimension = dimension

    @classmethod
    def memory(
        cls,
        dimension: int,
        *,
        backend: ExecutionMode = "cpu",
        gpu_filter_mode: GpuFilterMode = "gpu-predicate",
        gpu_allocation_budget_bytes: int = _DEFAULT_GPU_BUDGET_BYTES,
    ) -> Collection:
        """Create an in-memory collection with an explicit execution policy."""
        values = cls._config_values(
            dimension, backend, gpu_filter_mode, gpu_allocation_budget_bytes
        )
        return cls(LIB.qenlo_collection_new_configured(*values), dimension)

    @classmethod
    def create(
        cls,
        path: str | PathLike[str],
        dimension: int,
        *,
        backend: ExecutionMode = "cpu",
        gpu_filter_mode: GpuFilterMode = "gpu-predicate",
        gpu_allocation_budget_bytes: int = _DEFAULT_GPU_BUDGET_BYTES,
    ) -> Collection:
        """Create durable state in a new or empty directory."""
        values = cls._config_values(
            dimension, backend, gpu_filter_mode, gpu_allocation_budget_bytes
        )
        return cls(
            LIB.qenlo_collection_create_configured(str(path).encode(), *values), dimension
        )

    @classmethod
    def open(
        cls,
        path: str | PathLike[str],
        dimension: int,
        *,
        backend: ExecutionMode = "cpu",
        gpu_filter_mode: GpuFilterMode = "gpu-predicate",
        gpu_allocation_budget_bytes: int = _DEFAULT_GPU_BUDGET_BYTES,
    ) -> Collection:
        """Open previously created durable state under an exclusive process lock."""
        values = cls._config_values(
            dimension, backend, gpu_filter_mode, gpu_allocation_budget_bytes
        )
        return cls(LIB.qenlo_collection_open_configured(str(path).encode(), *values), dimension)

    @classmethod
    def import_qn(
        cls,
        path: str | PathLike[str],
        dimension: int,
        *,
        backend: ExecutionMode = "cpu",
        gpu_filter_mode: GpuFilterMode = "gpu-predicate",
        gpu_allocation_budget_bytes: int = _DEFAULT_GPU_BUDGET_BYTES,
    ) -> Collection:
        """Import a checksummed `.qn` snapshot into a mutable in-memory collection."""
        values = cls._config_values(
            dimension, backend, gpu_filter_mode, gpu_allocation_budget_bytes
        )
        return cls(
            LIB.qenlo_collection_import_qn_configured(str(path).encode(), *values), dimension
        )

    @staticmethod
    def _validate_dimension(dimension: int) -> None:
        if dimension <= 0:
            raise ValueError("dimension must be positive")

    @classmethod
    def _config_values(
        cls,
        dimension: int,
        backend: ExecutionMode,
        gpu_filter_mode: GpuFilterMode,
        gpu_allocation_budget_bytes: int,
    ) -> tuple[int, int, int, int]:
        cls._validate_dimension(dimension)
        if backend not in _BACKENDS:
            raise ValueError(f"unknown backend {backend!r}")
        if gpu_filter_mode not in _GPU_FILTERS:
            raise ValueError(f"unknown GPU filter mode {gpu_filter_mode!r}")
        _validate_u64(gpu_allocation_budget_bytes, "gpu_allocation_budget_bytes")
        return (
            dimension,
            _BACKENDS[backend],
            _GPU_FILTERS[gpu_filter_mode],
            gpu_allocation_budget_bytes,
        )

    def _require_open(self) -> Handle:
        if self._handle is None:
            raise QenloError("collection is closed")
        return self._handle

    def _vector(self, vector: Sequence[float]) -> ctypes.Array[ctypes.c_float]:
        if len(vector) != self.dimension:
            raise ValueError(f"expected vector dimension {self.dimension}, got {len(vector)}")
        return (ctypes.c_float * self.dimension)(*vector)

    @staticmethod
    def _check(status: int) -> None:
        if status != 0:
            raise QenloError(last_error())

    def add(self, record: Record) -> None:
        """Validate, normalize, and atomically add one record."""
        _validate_u64(record.id, "id")
        _validate_u64(record.user_id, "user_id")
        _validate_i64(record.timestamp, "timestamp")
        vector = self._vector(record.vector)
        self._check(
            LIB.qenlo_add(
                self._require_open(), record.id, record.user_id, record.timestamp, vector, self.dimension
            )
        )

    def add_batch(self, records: Iterable[Record]) -> None:
        """Add every record in one ordered atomic transaction."""
        rows = tuple(records)
        if not rows:
            return
        ids = (ctypes.c_uint64 * len(rows))()
        users = (ctypes.c_uint64 * len(rows))()
        timestamps = (ctypes.c_int64 * len(rows))()
        vectors = (ctypes.c_float * (len(rows) * self.dimension))()
        for row, record in enumerate(rows):
            _validate_u64(record.id, "id")
            _validate_u64(record.user_id, "user_id")
            _validate_i64(record.timestamp, "timestamp")
            if len(record.vector) != self.dimension:
                raise ValueError(f"expected vector dimension {self.dimension}, got {len(record.vector)}")
            ids[row], users[row], timestamps[row] = record.id, record.user_id, record.timestamp
            for column, value in enumerate(record.vector):
                vectors[row * self.dimension + column] = value
        self._check(
            LIB.qenlo_add_batch(
                self._require_open(), ids, users, timestamps, vectors, len(rows), self.dimension
            )
        )

    def add_buffer(self, vectors, ids: Sequence[int], *, user_ids: Sequence[int],
                   timestamps: Sequence[int]) -> None:
        """Atomically ingest a C-contiguous native float32 matrix.

        Accepts NumPy arrays or another buffer provider without copying each
        vector component through Python. Read-only exporters incur one bulk copy;
        writable exporters are borrowed for the call. Native code owns a copy before
        return. Do not mutate writable buffers concurrently with this call.
        """
        view = memoryview(vectors)
        if view.format != "f" or view.itemsize != 4 or not view.c_contiguous:
            raise ValueError("vectors must be a C-contiguous native float32 buffer")
        if view.ndim != 2 or view.shape != (len(ids), self.dimension):
            raise ValueError("vectors must have shape (len(ids), dimension)")
        if len(user_ids) != len(ids) or len(timestamps) != len(ids):
            raise ValueError("one user_id and timestamp are required per ID")
        for value in ids:
            _validate_u64(value, "id")
        for value in user_ids:
            _validate_u64(value, "user_id")
        for value in timestamps:
            _validate_i64(value, "timestamp")
        handle = self._require_open()
        if not len(ids):
            return
        array_type = ctypes.c_float * (len(ids) * self.dimension)
        data = array_type.from_buffer_copy(view) if view.readonly else array_type.from_buffer(view)
        self._check(LIB.qenlo_add_batch(
            handle, (ctypes.c_uint64 * len(ids))(*ids),
            (ctypes.c_uint64 * len(ids))(*user_ids),
            (ctypes.c_int64 * len(ids))(*timestamps), data, len(ids), self.dimension))

    def delete(self, id: int) -> None:
        """Delete one live record. IDs are never reusable."""
        _validate_u64(id, "id")
        self._check(LIB.qenlo_delete(self._require_open(), id))

    def delete_batch(self, ids: Iterable[int]) -> None:
        """Delete every ID in one ordered atomic transaction."""
        values = tuple(ids)
        if not values:
            return
        for id in values:
            _validate_u64(id, "id")
        native = (ctypes.c_uint64 * len(values))(*values)
        self._check(LIB.qenlo_delete_batch(self._require_open(), native, len(values)))

    def search(self, query: Sequence[float], filter: Filter = Filter(), k: int = 10) -> SearchResponse:
        """Return distance-then-ID ordered hits and an execution report."""
        if not 1 <= k <= 64:
            raise ValueError("k must be in 1..=64")
        if filter.user_id is not None:
            _validate_u64(filter.user_id, "filter.user_id")
        if filter.timestamp_lower is not None:
            _validate_i64(filter.timestamp_lower, "filter.timestamp_lower")
        if filter.timestamp_upper is not None:
            _validate_i64(filter.timestamp_upper, "filter.timestamp_upper")
        vector = self._vector(query)
        results = LIB.qenlo_search_results_new(
            self._require_open(),
            vector,
            self.dimension,
            filter.user_id is not None,
            filter.user_id or 0,
            filter.timestamp_lower is not None,
            filter.timestamp_lower or 0,
            filter.timestamp_upper is not None,
            filter.timestamp_upper or 0,
            k,
        )
        if not results:
            raise QenloError(last_error())
        try:
            rows = ctypes.c_size_t()
            self._check(LIB.qenlo_search_results_len(results, ctypes.byref(rows)))
            ids = (ctypes.c_uint64 * rows.value)()
            distances = (ctypes.c_float * rows.value)()
            self._check(
                LIB.qenlo_search_results_copy(
                    results, ids, rows.value, distances, rows.value
                )
            )
            report: dict[str, Any] = json.loads(
                take_string(LIB.qenlo_search_results_report_json(results))
            )
        finally:
            LIB.qenlo_search_results_free(results)
        for name in (
            "operation_id",
            "index_generation",
            "total_duration_ns",
            "lock_wait_ns",
            "eligible_rows",
            "upload_bytes",
            "readback_bytes",
            "allocation_bytes",
            "candidates",
        ):
            if report[name] is not None:
                report[name] = int(report[name])
        return SearchResponse(
            results=tuple(
                SearchResult(id=int(ids[row]), distance=float(distances[row]))
                for row in range(rows.value)
            ),
            report=ExecutionReport(**report),
        )

    def stats(self) -> CollectionStats:
        """Return canonical, durable, and lifecycle state without row payloads."""
        value = json.loads(take_string(LIB.qenlo_stats(self._require_open())))
        for name in ("generation", "prepared_generation", "durable_generation"):
            if value[name] is not None:
                value[name] = int(value[name])
        return CollectionStats(**value)

    def _generation(self) -> int:
        generation = ctypes.c_uint64()
        self._check(LIB.qenlo_collection_generation(self._require_open(), ctypes.byref(generation)))
        return int(generation.value)

    def _snapshot_buffer(self, filter: Filter = Filter()) -> _CanonicalBuffer:
        if filter.user_id is not None:
            _validate_u64(filter.user_id, "filter.user_id")
        if filter.timestamp_lower is not None:
            _validate_i64(filter.timestamp_lower, "filter.timestamp_lower")
        if filter.timestamp_upper is not None:
            _validate_i64(filter.timestamp_upper, "filter.timestamp_upper")
        snapshot = LIB.qenlo_snapshot_new(
            self._require_open(),
            filter.user_id is not None, filter.user_id or 0,
            filter.timestamp_lower is not None, filter.timestamp_lower or 0,
            filter.timestamp_upper is not None, filter.timestamp_upper or 0,
        )
        if not snapshot:
            raise QenloError(last_error())
        try:
            generation = ctypes.c_uint64()
            rows = ctypes.c_size_t()
            dimension = ctypes.c_size_t()
            self._check(LIB.qenlo_snapshot_info(
                snapshot, ctypes.byref(generation), ctypes.byref(rows), ctypes.byref(dimension)
            ))
            ids = (ctypes.c_uint64 * rows.value)()
            vector_count = rows.value * dimension.value
            vectors = (ctypes.c_float * vector_count)()
            self._check(LIB.qenlo_snapshot_copy(
                snapshot, ids, rows.value, vectors, vector_count
            ))
            return _CanonicalBuffer(
                int(generation.value), int(dimension.value), int(rows.value), ids, vectors
            )
        finally:
            LIB.qenlo_snapshot_free(snapshot)

    def flush(self) -> None:
        """Compact durable WAL state into a canonical snapshot."""
        self._check(LIB.qenlo_flush(self._require_open()))

    def export_qn(self, path: str | PathLike[str]) -> None:
        """Atomically export the current generation to a new portable `.qn` file."""
        self._check(LIB.qenlo_export_qn(self._require_open(), str(path).encode()))

    def close(self) -> None:
        """Close and release the native collection. Idempotent."""
        if self._handle is not None:
            handle, self._handle = self._handle, None
            status = LIB.qenlo_close(handle)
            LIB.qenlo_collection_free(handle)
            self._check(status)

    def __enter__(self) -> Collection:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()

    def __del__(self) -> None:
        if getattr(self, "_handle", None) is not None:
            LIB.qenlo_collection_free(self._handle)
            self._handle = None
