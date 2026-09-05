"""Small CPU-only conformance checks; GPU matrices run separately on Runpod."""
import numpy as np
import pytest
torch = pytest.importorskip("torch")

from qenlo import Collection, Filter, Record, TorchIndex

torch.set_num_threads(1)


def test_oracle_batch_ties_and_owned_inputs():
    rng = np.random.default_rng(42)
    vectors = rng.normal(size=(521, 33)).astype('float32')
    queries = rng.normal(size=(7, 33)).astype('float32')
    ids = rng.permutation(521).astype('int64')
    index = TorchIndex(vectors, ids)
    oracle_vectors = vectors.astype('float64')
    oracle_vectors /= np.linalg.norm(oracle_vectors, axis=1, keepdims=True)
    oracle_queries = queries.astype('float64')
    oracle_queries /= np.linalg.norm(oracle_queries, axis=1, keepdims=True)
    expected = np.array([ids[np.lexsort((ids, 1 - oracle_vectors @ q))[:64]]
                         for q in oracle_queries])
    vectors[:] = 0
    got, distances = index.search(queries, 64)
    assert np.array_equal(got.numpy(), expected)
    assert bool((distances[:, 1:] >= distances[:, :-1]).all())
    tied = TorchIndex([[1., 0.]] * 300, torch.arange(300).flip(0))
    assert tied.search([1., 0.], 64)[0].tolist() == [list(range(64))]
    assert TorchIndex(torch.empty(0, 2)).search([1., 0.])[0].shape == (1, 0)
    assert TorchIndex([[3e38, 3e38]]).search([3e38, 3e38])[0].item() == 0


def test_validation_and_no_query_mutation():
    for vectors, ids in [([[0., 0.]], None), ([[float('nan'), 1.]], None),
                         ([[1., 0.]], [1.5]), ([[1., 0.], [0., 1.]], [1, 1])]:
        with pytest.raises(ValueError):
            TorchIndex(vectors, ids)
    index = TorchIndex([[1., 0.]])
    query = torch.tensor([2., 0.], requires_grad=True)
    index.search(query)
    assert query.tolist() == [2., 0.]
    for query, k in [([0., 0.], 1), ([1.], 1), ([1., 0.], True), ([1., 0.], 65)]:
        with pytest.raises(ValueError):
            index.search(query, k)


def test_collection_snapshot_filter_and_generation_binding():
    with Collection.memory(2) as collection:
        collection.add_batch([
            Record(9, 7, -1, [1.0, 0.0]),
            Record(2, 7, 1, [1.0, 0.0]),
            Record(4, 8, 1, [0.0, 1.0]),
        ])
        index = TorchIndex.from_collection(collection, Filter(user_id=7))
        assert index.search([1.0, 0.0], 10)[0].tolist() == [[2, 9]]
        assert index.allocation_bytes == 2 * (2 * 4 + 8)
        collection.delete(2)
        with pytest.raises(RuntimeError, match="stale"):
            index.search([1.0, 0.0])


def test_device_id_and_budget_limits_are_explicit():
    with pytest.raises(ValueError, match="CPU, CUDA, or MPS"):
        TorchIndex([[1.0]], device="meta")
    if not torch.cuda.is_available():
        with pytest.raises(RuntimeError, match="CUDA"):
            TorchIndex([[1.0]], device="cuda")
    with pytest.raises(ValueError, match=r"0\.\.=2\^63-1"):
        TorchIndex([[1.0]], [2**64 - 1])
    with pytest.raises(MemoryError, match="resident tensors"):
        TorchIndex([[1.0, 0.0]], [1], max_bytes=15)
    index = TorchIndex([[1.0, 0.0]], [1], max_bytes=32)
    with pytest.raises(MemoryError, match="search requires"):
        index.search([1.0, 0.0])
