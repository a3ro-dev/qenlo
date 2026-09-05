"""Qenlo's native collection and optional resident tensor index.

Native code and PyTorch are loaded only when their respective APIs are used.
"""
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._collection import (Collection, CollectionStats, ExecutionMode,
                             ExecutionReport, Filter, GpuFilterMode, QenloError,
                             Record, SearchResponse, SearchResult)
    from .torch import TorchIndex

__all__ = ["Collection", "CollectionStats", "ExecutionMode", "ExecutionReport",
           "Filter", "GpuFilterMode", "QenloError", "Record", "SearchResponse",
           "SearchResult", "TorchIndex"]


def __getattr__(name: str):
    if name not in __all__:
        raise AttributeError(name)
    if name == "TorchIndex":
        from .torch import TorchIndex
        value = TorchIndex
    else:
        from . import _collection
        value = getattr(_collection, name)
    globals()[name] = value
    return value
