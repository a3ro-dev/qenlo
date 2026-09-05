from __future__ import annotations

import math
from array import array

import pytest

from qenlo import Collection, Filter, QenloError, Record


def fixture() -> tuple[Record, ...]:
    return (
        Record(9, 7, -5, (1.0, 0.0, 0.0)),
        Record(2, 7, 0, (2.0, 0.0, 0.0)),
        Record(4, 8, 10, (0.0, 1.0, 0.0)),
        Record(6, 7, 20, (0.0, 0.0, 1.0)),
    )


def test_typed_search_filter_ordering_and_execution_report() -> None:
    with Collection.memory(3) as db:
        db.add_batch(fixture())
        result = db.search(
            (1.0, 0.0, 0.0),
            Filter(user_id=7, timestamp_lower=-5, timestamp_upper=20),
            10,
        )
        assert [hit.id for hit in result.results] == [2, 9]
        assert result.report.actual_backend == "Cpu"
        assert result.report.algorithm == "Exact"
        assert result.report.operation_id > 0
        assert db.stats().live_rows == 4


def test_execution_configuration_validation() -> None:
    with pytest.raises(ValueError, match="unknown backend"):
        Collection.memory(3, backend="missing")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="unknown GPU filter"):
        Collection.memory(3, gpu_filter_mode="missing")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="unsigned 64-bit"):
        Collection.memory(3, gpu_allocation_budget_bytes=-1)


def test_atomic_batches_and_non_reusable_ids() -> None:
    with Collection.memory(3) as db:
        db.add(fixture()[0])
        with pytest.raises(QenloError):
            db.add_batch((fixture()[1], fixture()[0]))
        assert db.stats().rows == 1
        db.delete(9)
        with pytest.raises(QenloError):
            db.add(fixture()[0])


def test_bulk_buffer_owns_values_and_accepts_read_only_exporters() -> None:
    values = array("f", [1.0, 0.0, 0.0, 0.0, 1.0, 0.0])
    writable = memoryview(values).cast("B").cast("f", shape=(2, 3))
    read_only = memoryview(array("f", [0.0, 0.0, 1.0]).tobytes()).cast(
        "f", shape=(1, 3)
    )
    with Collection.memory(3) as db:
        db.add_buffer(writable, [9, 2], user_ids=[7, 7], timestamps=[-5, 0])
        values[:] = array("f", [0.0] * 6)
        db.add_buffer(read_only, [4], user_ids=[8], timestamps=[10])
        assert [hit.id for hit in db.search((1.0, 0.0, 0.0)).results] == [9, 2, 4]


def test_bulk_buffer_rejects_atomically_before_or_inside_native_call() -> None:
    values = memoryview(array("f", [1.0, 0.0, 0.0, 0.0, 1.0, 0.0])).cast(
        "B"
    ).cast("f", shape=(2, 3))
    with Collection.memory(3) as db:
        for kwargs in [
            dict(ids=[1, 2], user_ids=[1], timestamps=[0, 0]),
            dict(ids=[1, -1], user_ids=[1, 1], timestamps=[0, 0]),
            dict(ids=[1, 1], user_ids=[1, 1], timestamps=[0, 0]),
        ]:
            with pytest.raises((ValueError, QenloError)):
                db.add_buffer(values, **kwargs)
            assert db.stats().rows == 0


def test_durable_reopen_and_delete_batch(tmp_path) -> None:
    path = tmp_path / "vectors.qenlo"
    with Collection.create(path, 3) as db:
        db.add_batch(fixture())
        db.delete_batch((2, 4))
        db.flush()
    with Collection.open(path, 3) as db:
        assert db.stats().live_rows == 2
        assert [hit.id for hit in db.search((1.0, 0.0, 0.0)).results] == [9, 6]


def test_portable_qn_round_trip(tmp_path) -> None:
    path = tmp_path / "vectors.qn"
    with Collection.memory(3) as db:
        db.add_batch(fixture())
        db.delete(9)
        db.export_qn(path)
        with pytest.raises(QenloError, match="already exists"):
            db.export_qn(path)
    assert path.is_file()
    with Collection.import_qn(path, 3) as imported:
        assert imported.stats().generation == 5
        assert imported.stats().live_rows == 3
        assert [hit.id for hit in imported.search((1.0, 0.0, 0.0)).results] == [2, 4, 6]


@pytest.mark.parametrize(
    "vector",
    [(), (1.0,), (1.0, 0.0), (1.0, 0.0, 0.0, 0.0)],
)
def test_dimension_validation(vector: tuple[float, ...]) -> None:
    with Collection.memory(3) as db, pytest.raises(ValueError):
        db.add(Record(1, 1, 0, vector))


@pytest.mark.parametrize("k", [-1, 0, 65, 100])
def test_k_validation(k: int) -> None:
    with Collection.memory(3) as db, pytest.raises(ValueError):
        db.search((1.0, 0.0, 0.0), k=k)


@pytest.mark.parametrize("vector", [(0.0, 0.0, 0.0), (math.nan, 0.0, 0.0), (math.inf, 0.0, 0.0)])
def test_native_vector_validation(vector: tuple[float, ...]) -> None:
    with Collection.memory(3) as db, pytest.raises(QenloError):
        db.add(Record(1, 1, 0, vector))


def test_closed_collection_fails_without_native_access() -> None:
    db = Collection.memory(3)
    db.close()
    db.close()
    with pytest.raises(QenloError, match="closed"):
        db.stats()
