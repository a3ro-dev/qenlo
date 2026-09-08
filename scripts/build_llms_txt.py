"""Build llms.txt and llms-full.txt for Qenlo documentation."""

from pathlib import Path

BASE_URL = "https://a3ro-dev.github.io/qenlo"

DOCS_CATALOG = [
    ("Getting started", [
        ("docs/README.md", "Overview", "Core introduction, boundaries, and documentation index"),
        ("docs/quickstart.md", "Quickstart", "Build from source, create collections, insert, search, and reopen"),
        ("docs/concepts.md", "Core concepts", "Canonical state, eligibility, exactness, and routing boundaries"),
        ("docs/browser.md", "Collection browser", "Terminal user interface (TUI), local web UI, and Tauri desktop inspector"),
        ("docs/use-cases.md", "Use cases", "Where Qenlo fits and where it does not"),
        ("docs/trade-offs.md", "Trade-offs", "Memory costs, CPU baseline realities, and routing guidance"),
        ("docs/feature-matrix.md", "Feature matrix", "Implemented, optional, and unverified capabilities"),
    ]),
    ("SDKs and bindings", [
        ("docs/sdks/rust.md", "Rust API", "Core asynchronous Rust client, options, and usage"),
        ("docs/sdks/python.md", "Python SDK", "Python bindings, bulk buffer ingestion, and optional TorchIndex"),
        ("docs/sdks/typescript.md", "TypeScript SDK", "Node.js native FFI bindings with TypeScript type safety"),
        ("docs/sdks/go.md", "Go driver", "cgo bindings for Go services and command-line tools"),
        ("docs/sdks/kotlin.md", "Kotlin SDK", "JNA bindings for JVM applications"),
        ("docs/sdks/swift.md", "Swift SDK", "Swift bindings over the shared C ABI"),
    ]),
    ("Architecture and storage", [
        ("docs/architecture.md", "Architecture specification", "CoreStore, transaction lifecycle, lock protocols, and visibility"),
        ("docs/qn-format-v1.md", ".qn storage format", "Binary file layout, little-endian specs, and atomic exchange semantics"),
        ("docs/recovery-policy.md", "Recovery policy", "Deterministic recovery, fail-closed boundaries, and operator contracts"),
        ("docs/compatibility.md", "Compatibility policy", "Storage, SDK, ABI, and prerelease change guarantees"),
        ("docs/gpu-design.md", "GPU search design", "Custom WGSL scoring, selection pipelines, chunking, and memory arenas"),
        ("docs/cuda-backend-todo.md", "CUDA backend todo", "Entry gates and engineering requirements for a native CUDA backend"),
    ]),
    ("Research and verification", [
        ("docs/benchmark-protocol.md", "Benchmark protocol", "Reproducible test harness execution and cell definitions"),
        ("docs/device-lab.md", "Device lab", "Multi-platform test runner, profiles, and telemetry handling"),
        ("docs/results-2026-08-28.md", "Measured results (2026-08-28)", "Native Windows benchmark run data and comparisons"),
        ("docs/verification.md", "Verification record", "Verification record, test outcomes, and audit trail"),
        ("docs/implementation-status.md", "Implementation status", "Engineering roadmap status and hardware gates"),
        ("docs/prioritized-roadmap.md", "Prioritized roadmap", "Ranked adoption, trust, mobile, and research milestones"),
        ("docs/ci.md", "CI and release map", "GitHub Actions workflows and release sequence"),
        ("docs/publishing.md", "Package publishing", "Multi-registry publication pipeline"),
    ]),
]


def build_llms_txt() -> str:
    lines = [
        "# Qenlo",
        "",
        "> Embedded filtered vector search for native applications. Measured before marketed.",
        "",
        "Qenlo is a research-grade embedded vector database for durable, exact, metadata-filtered retrieval. It runs in-process with canonical local records and optional CPU, WGPU, and tensor execution.",
        "",
        "- Website: https://a3ro-dev.github.io/qenlo/",
        "- Documentation: https://a3ro-dev.github.io/qenlo/docs/",
        "- Repository: https://github.com/a3ro-dev/qenlo",
        "",
    ]

    for section_title, docs in DOCS_CATALOG:
        lines.append(f"## {section_title}")
        lines.append("")
        for rel_path, title, desc in docs:
            url = f"{BASE_URL}/{rel_path}"
            lines.append(f"- [{title}]({url}): {desc}")
        lines.append("")

    lines.append("## Optional")
    lines.append("")
    lines.append(f"- [Consolidated full documentation bundle]({BASE_URL}/llms-full.txt): Complete documentation suite concatenated into a single plain text file")
    lines.append("")

    return "\n".join(lines)


def build_llms_full_txt(root_dir: Path) -> str:
    sections = [
        "# Qenlo: Complete Documentation Bundle",
        "",
        "This file concatenates all official Qenlo documentation pages for AI agents and language models.",
        "Project repository: https://github.com/a3ro-dev/qenlo",
        "Project documentation: https://a3ro-dev.github.io/qenlo/docs/",
        "",
    ]

    for section_title, docs in DOCS_CATALOG:
        for rel_path, title, desc in docs:
            file_path = root_dir / rel_path
            if not file_path.exists():
                continue
            text = file_path.read_text(encoding="utf-8").strip()
            url = f"{BASE_URL}/{rel_path}"

            header = [
                "=" * 80,
                f"Document: {title}",
                f"Path: {rel_path}",
                f"URL: {url}",
                f"Description: {desc}",
                "=" * 80,
                "",
                text,
                "",
                "",
            ]
            sections.extend(header)

    return "\n".join(sections)


def main():
    root = Path(__file__).resolve().parent.parent
    llms_txt = build_llms_txt()
    llms_full = build_llms_full_txt(root)

    # 1. Root files
    def write_lf(path: Path, text: str) -> None:
        with path.open("w", encoding="utf-8", newline="\n") as stream:
            stream.write(text)

    write_lf(root / "llms.txt", llms_txt)
    write_lf(root / "llms-full.txt", llms_full)

    # 2. docs/ copies
    write_lf(root / "docs" / "llms.txt", llms_txt)
    write_lf(root / "docs" / "llms-full.txt", llms_full)

    # 3. .well-known/ copy
    well_known = root / ".well-known"
    well_known.mkdir(exist_ok=True)
    write_lf(well_known / "llms.txt", llms_txt)

    print("Generated llms.txt and llms-full.txt across root, docs/, and .well-known/")


if __name__ == "__main__":
    main()
