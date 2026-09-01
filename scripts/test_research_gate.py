import csv
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from research_gate import GATE, evaluate, validate_runtime, validate_shape


class ResearchGateChecks(unittest.TestCase):
    def run_record(self, directory, backend="cpu", latency=100, passed=True):
        return {
            "directory": str(directory),
            "backend_requested": backend,
            "median_run_p95_batch_ns": latency,
            "workload": dict(GATE),
            "recall_gate": {"passed": passed},
        }

    def test_shape_is_immutable(self):
        run = self.run_record(Path("cpu"))
        validate_shape(run, "cpu")
        for key in GATE:
            changed = self.run_record(Path("changed"))
            value = changed["workload"][key]
            changed["workload"][key] = value + 1 if isinstance(value, int) else "changed"
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate_shape(changed, "changed")

    def test_runtime_rejects_fallback_backend_and_missing_telemetry(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "configuration.txt").write_text(
                "source_kind=imported-raw-f32-le\ndiagnostics=basic\ngit_worktree_dirty=false\n"
                "gpu_adapter=x\ngpu_api=Vulkan\ngpu_device_type=DiscreteGpu\n",
                encoding="utf-8",
            )
            fields = ["actual_backend", "fallback", "eligible_count", "upload_bytes", "readback_bytes", "max_qenlo_allocation_bytes"]
            with (directory / "samples.csv").open("w", newline="", encoding="utf-8") as stream:
                writer = csv.DictWriter(stream, fieldnames=fields); writer.writeheader()
                row = dict(actual_backend="Wgpu", fallback="false", eligible_count="10000", upload_bytes="1", readback_bytes="1", max_qenlo_allocation_bytes="1")
                for _ in range(25_000): writer.writerow(row)
            validate_runtime(directory, "Wgpu", require_gpu=True)
            rows = (directory / "samples.csv").read_text(encoding="utf-8").replace("Wgpu,false", "Cpu,false", 1)
            (directory / "samples.csv").write_text(rows, encoding="utf-8")
            with self.assertRaises(ValueError): validate_runtime(directory, "Wgpu", require_gpu=True)

    @patch("research_gate.validate_runtime", return_value={"gpu_adapter": "x", "gpu_api": "Vulkan", "gpu_device_type": "DiscreteGpu"})
    @patch("research_gate.compare")
    @patch("research_gate.read_run")
    def test_fastest_qualifying_cpu_and_exact_two_x_rule(self, read_run, compare, _runtime):
        cpu, ann, gpu = Path("cpu"), Path("ann"), Path("gpu")
        records = {
            cpu: self.run_record(cpu, "cpu", 100),
            ann: self.run_record(ann, "usearch", 80),
            gpu: self.run_record(gpu, "gpu-predicate", 40),
        }
        read_run.side_effect = lambda path: records[path]
        compare.return_value = {"latency_comparison_valid": True, "baseline_over_candidate_median_p95_ratio": 2.0}
        result = evaluate([cpu, ann], gpu)
        self.assertEqual(result["status"], "passed")
        self.assertEqual(result["selected_cpu_baseline"]["backend_requested"], "usearch")
        compare.return_value["baseline_over_candidate_median_p95_ratio"] = 1.999
        self.assertEqual(evaluate([cpu, ann], gpu)["status"], "failed")


if __name__ == "__main__":
    unittest.main()
