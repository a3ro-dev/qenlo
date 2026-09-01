"""Type-safe Python API for Qenlo."""

from __future__ import annotations

import ctypes
import json
from dataclasses import dataclass
from os import PathLike
from typing import Any, Iterable, Mapping, Sequence

from ._native import LIB, Handle, last_error, take_string

__all__ = [
    "Collection",
    "CollectionStats",
    "ExecutionReport",
    "Filter",
    "QenloError",
    "Record",
    "SearchResponse",
    "SearchResult",
]


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
    def memory(cls, dimension: int) -> Collection:
        """Create an in-memory exact-CPU collection."""
        cls._validate_dimension(dimension)
        return cls(LIB.qenlo_collection_new(dimension), dimension)

    @classmethod
    def create(cls, path: str | PathLike[str], dimension: int) -> Collection:
        """Create durable state in a new or empty directory."""
        cls._validate_dimension(dimension)
        return cls(LIB.qenlo_collection_create(str(path).encode(), dimension), dimension)

    @classmethod
    def open(cls, path: str | PathLike[str], dimension: int) -> Collection:
        """Open previously created durable state under an exclusive process lock."""
        cls._validate_dimension(dimension)
        return cls(LIB.qenlo_collection_open(str(path).encode(), dimension), dimension)

    @staticmethod
    def _validate_dimension(dimension: int) -> None:
        if dimension <= 0:
            raise ValueError("dimension must be positive")

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
        raw = take_string(
            LIB.qenlo_search(
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
        )
        value: Mapping[str, Any] = json.loads(raw)
        report = dict(value["report"])
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
            results=tuple(SearchResult(id=int(hit["id"]), distance=hit["distance"]) for hit in value["results"]),
            report=ExecutionReport(**report),
        )

    def stats(self) -> CollectionStats:
        """Return canonical, durable, and lifecycle state without row payloads."""
        value = json.loads(take_string(LIB.qenlo_stats(self._require_open())))
        for name in ("generation", "prepared_generation", "durable_generation"):
            if value[name] is not None:
                value[name] = int(value[name])
        return CollectionStats(**value)

    def flush(self) -> None:
        """Compact durable WAL state into a canonical snapshot."""
        self._check(LIB.qenlo_flush(self._require_open()))

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
