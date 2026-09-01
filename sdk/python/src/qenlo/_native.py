from __future__ import annotations

import ctypes
import os
import platform
from pathlib import Path
from typing import Final


def _library_name() -> str:
    system = platform.system()
    if system == "Windows":
        return "qenlo_ffi.dll"
    if system == "Darwin":
        return "libqenlo_ffi.dylib"
    return "libqenlo_ffi.so"


def _library_path() -> Path:
    explicit = os.environ.get("QENLO_LIBRARY_PATH")
    if explicit:
        return Path(explicit)
    package = Path(__file__).parent / "native" / _library_name()
    if package.is_file():
        return package
    repository = Path(__file__).resolve().parents[4] / "target" / "release" / _library_name()
    if repository.is_file():
        return repository
    raise RuntimeError(
        f"Qenlo native library not found. Set QENLO_LIBRARY_PATH or install a platform wheel ({_library_name()})."
    )


LIB: Final = ctypes.CDLL(str(_library_path()))
Handle = ctypes.c_void_p

LIB.qenlo_collection_new.argtypes = [ctypes.c_size_t]
LIB.qenlo_collection_new.restype = Handle
LIB.qenlo_collection_create.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
LIB.qenlo_collection_create.restype = Handle
LIB.qenlo_collection_open.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
LIB.qenlo_collection_open.restype = Handle
LIB.qenlo_add.argtypes = [
    Handle,
    ctypes.c_uint64,
    ctypes.c_uint64,
    ctypes.c_int64,
    ctypes.POINTER(ctypes.c_float),
    ctypes.c_size_t,
]
LIB.qenlo_add.restype = ctypes.c_int32
LIB.qenlo_add_batch.argtypes = [
    Handle,
    ctypes.POINTER(ctypes.c_uint64),
    ctypes.POINTER(ctypes.c_uint64),
    ctypes.POINTER(ctypes.c_int64),
    ctypes.POINTER(ctypes.c_float),
    ctypes.c_size_t,
    ctypes.c_size_t,
]
LIB.qenlo_add_batch.restype = ctypes.c_int32
LIB.qenlo_delete.argtypes = [Handle, ctypes.c_uint64]
LIB.qenlo_delete.restype = ctypes.c_int32
LIB.qenlo_delete_batch.argtypes = [Handle, ctypes.POINTER(ctypes.c_uint64), ctypes.c_size_t]
LIB.qenlo_delete_batch.restype = ctypes.c_int32
LIB.qenlo_search.argtypes = [
    Handle,
    ctypes.POINTER(ctypes.c_float),
    ctypes.c_size_t,
    ctypes.c_bool,
    ctypes.c_uint64,
    ctypes.c_bool,
    ctypes.c_int64,
    ctypes.c_bool,
    ctypes.c_int64,
    ctypes.c_size_t,
]
LIB.qenlo_search.restype = ctypes.c_void_p
LIB.qenlo_stats.argtypes = [Handle]
LIB.qenlo_stats.restype = ctypes.c_void_p
LIB.qenlo_flush.argtypes = [Handle]
LIB.qenlo_flush.restype = ctypes.c_int32
LIB.qenlo_close.argtypes = [Handle]
LIB.qenlo_close.restype = ctypes.c_int32
LIB.qenlo_collection_free.argtypes = [Handle]
LIB.qenlo_collection_free.restype = None
LIB.qenlo_last_error.argtypes = []
LIB.qenlo_last_error.restype = ctypes.c_void_p
LIB.qenlo_string_free.argtypes = [ctypes.c_void_p]
LIB.qenlo_string_free.restype = None


def take_string(pointer: int | None) -> str:
    if not pointer:
        raise RuntimeError(last_error())
    try:
        return ctypes.string_at(pointer).decode("utf-8")
    finally:
        LIB.qenlo_string_free(pointer)


def last_error() -> str:
    pointer = LIB.qenlo_last_error()
    if not pointer:
        return "unknown Qenlo native error"
    try:
        return ctypes.string_at(pointer).decode("utf-8")
    finally:
        LIB.qenlo_string_free(pointer)
