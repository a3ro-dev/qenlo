# pyright: reportMissingImports=false
"""Optional immutable FP32 cosine index for CPU, CUDA and macOS MPS tensors.

This is a derived in-memory index, not the durable Rust collection or a mobile
runtime. It owns its inputs. Rebuild it after canonical mutations. No network work.
"""
from __future__ import annotations

from numbers import Integral

import torch

_MAX_I64 = 2**63 - 1


class TorchIndex:
    """Resident exhaustive search for 1K–100K vectors; IDs are nonnegative int64.

    Inputs are copied and normalized at construction. Search returns (IDs,
    cosine distances) on the index device, shaped (batch, min(k, rows)).
    Computed distance ties use ascending ID, including ties at the cutoff.
    Caller-owned PyTorch precision settings are preserved.
    """

    @torch.inference_mode()
    def __init__(self, vectors, ids=None, *, device=None, max_bytes=None):
        target = torch.device(device) if device is not None else (
            vectors.device if isinstance(vectors, torch.Tensor) else torch.device("cpu")
        )
        if target.type not in {"cpu", "cuda", "mps"}:
            raise ValueError("device must be CPU, CUDA, or MPS")
        if target.type == "cuda" and not torch.cuda.is_available():
            raise RuntimeError("CUDA was requested but is not available")
        if target.type == "mps" and not torch.backends.mps.is_available():
            raise RuntimeError("MPS was requested but is not available")
        if max_bytes is not None and (
            isinstance(max_bytes, bool) or not isinstance(max_bytes, int) or max_bytes < 0
        ):
            raise ValueError("max_bytes must be a nonnegative integer or None")
        values = torch.as_tensor(vectors, dtype=torch.float32, device=target)
        if values.ndim != 2 or values.shape[1] == 0:
            raise ValueError("vectors must have shape (rows, dimension > 0)")
        values = values.detach().clone().contiguous()
        # Scale first: finite float32 inputs must not overflow the norm.
        scales = values.abs().amax(dim=1, keepdim=True)
        if not bool(torch.isfinite(values).all()) or bool((scales == 0).any()):
            raise ValueError("vectors must be finite and nonzero")
        values /= scales
        values /= torch.linalg.vector_norm(values, dim=1, keepdim=True)
        if ids is None:
            keys = torch.arange(len(values), dtype=torch.int64, device=values.device)
        else:
            if isinstance(ids, torch.Tensor):
                if ids.ndim != 1 or ids.dtype not in {torch.int64, torch.uint64}:
                    raise ValueError("ids must be a one-dimensional int64 or uint64 tensor")
                raw_ids = [int(value) for value in ids.tolist()]
            else:
                raw_ids = list(ids)
                if any(isinstance(value, bool) or not isinstance(value, Integral) for value in raw_ids):
                    raise ValueError("ids must contain integers (no implicit truncation)")
                raw_ids = [int(value) for value in raw_ids]
            if any(value < 0 or value > _MAX_I64 for value in raw_ids):
                raise ValueError(
                    "TorchIndex currently supports native IDs in 0..=2^63-1; "
                    "full uint64 tensors are not portable across CPU, CUDA, and MPS"
                )
            keys = torch.tensor(raw_ids, dtype=torch.int64, device=values.device)
            keys = keys.detach().clone()
        if keys.ndim != 1 or len(keys) != len(values):
            raise ValueError("one ID is required per vector")
        if bool((keys < 0).any()) or len(torch.unique(keys)) != len(keys):
            raise ValueError("ids must be unique nonnegative int64 values")
        order = torch.argsort(keys)
        self._ids = keys[order].contiguous()
        self._vectors = values[order].contiguous()
        self._allocation_bytes = (
            self._ids.numel() * self._ids.element_size()
            + self._vectors.numel() * self._vectors.element_size()
        )
        if max_bytes is not None and self._allocation_bytes > max_bytes:
            raise MemoryError(
                f"TorchIndex resident tensors require {self._allocation_bytes} bytes, "
                f"budget is {max_bytes} bytes"
            )
        self._max_bytes = max_bytes
        self._source_collection = None
        self._source_generation = None

    @classmethod
    def from_collection(cls, collection, filter=None, *, device=None, max_bytes=None):
        """Capture filtered live canonical rows and reject use after mutation."""
        if filter is None:
            from ._collection import Filter
            filter = Filter()
        captured = collection._snapshot_buffer(filter)
        vectors = (
            torch.frombuffer(captured.vectors, dtype=torch.float32).reshape(
                captured.rows, captured.dimension
            )
            if captured.rows
            else torch.empty((0, captured.dimension), dtype=torch.float32)
        )
        index = cls(vectors, list(captured.ids), device=device, max_bytes=max_bytes)
        index._source_collection = collection
        index._source_generation = captured.generation
        return index

    @property
    def device(self):
        return self._vectors.device

    @property
    def dimension(self):
        return self._vectors.shape[1]

    @property
    def allocation_bytes(self):
        """Bytes in owned ID/vector tensors; allocator and library overhead are excluded."""
        return self._allocation_bytes

    def __len__(self):
        return len(self._ids)

    @torch.inference_mode()
    def search(self, queries, k: int = 10):
        """Search up to 128 queries; includes validation and query transfer.

        For a repeated metadata filter, construct a separate index from the
        eligible live rows. Never reuse that index after a canonical mutation.
        """
        if isinstance(k, bool) or not isinstance(k, int) or not 1 <= k <= 64:
            raise ValueError("k must be an integer in 1..=64")
        if self._source_collection is not None:
            if self._source_collection._generation() != self._source_generation:
                raise RuntimeError("TorchIndex snapshot is stale after a canonical mutation")
        query = torch.as_tensor(queries, dtype=torch.float32, device=self.device)
        if query.ndim == 1:
            query = query.unsqueeze(0)
        if query.ndim != 2 or query.shape[1] != self.dimension or not 1 <= len(query) <= 128:
            raise ValueError("queries must have shape (1..128, dimension)")
        scales = query.abs().amax(dim=1, keepdim=True)
        if not bool(torch.isfinite(query).all()) or bool((scales == 0).any()):
            raise ValueError("queries must be finite and nonzero")
        query = query / scales
        query = query / torch.linalg.vector_norm(query, dim=1, keepdim=True)
        scratch_bytes = len(query) * (
            (self.dimension + len(self)) * 4 + min(k + 1, len(self)) * 12
        )
        if self._max_bytes is not None and self._allocation_bytes + scratch_bytes > self._max_bytes:
            raise MemoryError(
                f"TorchIndex search requires at least {self._allocation_bytes + scratch_bytes} "
                f"tracked tensor bytes, budget is {self._max_bytes} bytes"
            )
        # One GEMM for the batch; no per-query Python loop or JSON round trip.
        distances = 1.0 - query @ self._vectors.T
        count = min(k, len(self))
        if count == 0:
            return self._ids.expand(len(query), 0).clone(), distances
        values, positions = torch.topk(distances, min(count + 1, len(self)), largest=False)
        if count < len(self) and bool((values[:, count - 1] == values[:, count]).any()):
            # Sorted IDs plus a stable distance sort resolve even a fully tied corpus.
            positions = torch.argsort(distances, dim=1, stable=True)[:, :count]
        else:
            positions = positions[:, :count]
            positions = positions.gather(1, torch.argsort(positions, dim=1))
            ranked = distances.gather(1, positions)
            positions = positions.gather(1, torch.argsort(ranked, dim=1, stable=True))
        return self._ids[positions], distances.gather(1, positions)
