"""Make Qenlo's ctypes wheel explicitly platform-specific."""

from __future__ import annotations

import os
from typing import Any

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class CustomBuildHook(BuildHookInterface):
    """Apply the audited platform tag supplied by the release matrix."""

    def initialize(self, version: str, build_data: dict[str, Any]) -> None:
        del version
        tag = os.environ.get("QENLO_WHEEL_TAG")
        if not tag:
            raise RuntimeError(
                "QENLO_WHEEL_TAG is required because Qenlo wheels bundle a native library"
            )
        build_data["pure_python"] = False
        build_data["tag"] = tag
