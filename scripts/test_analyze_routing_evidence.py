import csv
import importlib.util
import tempfile
import unittest
from pathlib import Path

from analyze_routing_evidence import route_rows, summarize


class RoutingEvidenceTests(unittest.TestCase):
    def test_work_rule_fixes_high_batch_low_eligibility_misroute(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            matrix = root / "matrix.csv"
            fields = [
                "configuration", "workload", "engine", "status", "qualified",
                "rows", "eligible_fraction", "dimensions", "batch", "p95_completed_ns",
            ]
            with matrix.open("w", newline="", encoding="utf-8") as stream:
                writer = csv.DictWriter(stream, fieldnames=fields)
                writer.writeheader()
                common = {
                    "configuration": "gpu", "workload": "tiny-batch", "status": "completed",
                    "qualified": "True", "rows": "1000", "eligible_fraction": "0.01",
                    "dimensions": "384", "batch": "16",
                }
                writer.writerow(common | {"engine": "current-cpu", "p95_completed_ns": "100"})
                writer.writerow(common | {"engine": "current-gpu", "p95_completed_ns": "300"})
            crossover = root / "crossover.csv"
            crossover.write_text(
                "eligible,cpu_p95_ms,gpu_rows_p95_ms\n3000,1.0,0.8\n",
                encoding="utf-8",
            )
            rows = route_rows(matrix, crossover, 1_000_000)
            self.assertEqual(rows[0]["old_backend"], "gpu")
            self.assertEqual(rows[0]["candidate_backend"], "cpu")
            self.assertEqual(rows[1]["candidate_backend"], "gpu")
            self.assertTrue(summarize(rows, 1_000_000)["gate"]["development_evidence_passed"])


class ArchiveScalarTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = Path(__file__).resolve().parents[1] / "research/scripts/analyze_full_archive.py"
        spec = importlib.util.spec_from_file_location("archive_analysis", path)
        cls.analysis = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.analysis)

    def test_near_equal_work_alone_is_not_a_counterexample(self):
        rows = [
            {"name": "low", "k": 1, "work_units": 999, "winner": "cpu"},
            {"name": "high", "k": 1, "work_units": 1000, "winner": "gpu"},
        ]
        self.assertEqual(self.analysis.monotone_witnesses(rows), [])

    def test_inverted_work_with_same_k_is_a_counterexample(self):
        rows = [
            {"name": "low", "k": 10, "work_units": 999, "winner": "gpu"},
            {"name": "high", "k": 10, "work_units": 1000, "winner": "cpu"},
        ]
        self.assertEqual(len(self.analysis.monotone_witnesses(rows)), 1)
        rows[1]["k"] = 1
        self.assertEqual(self.analysis.monotone_witnesses(rows), [])

    def test_retained_gate_rejection_is_not_all_threshold_rejection(self):
        self.analysis.verify_archives()
        rows, summary = self.analysis.heldout_rows()
        self.assertEqual(summary["wrong_routes"], 8)
        self.assertGreater(summary["max_regret"], 0.25)
        self.assertEqual(summary["posthoc_minimax_threshold"]["wrong_routes"], 1)
        self.assertLess(summary["posthoc_minimax_threshold"]["max_regret"], 0.25)
        witnesses = self.analysis.monotone_witnesses(rows)
        self.assertTrue(witnesses)
        self.assertEqual({r["lower_work_gpu_winner"] for r in witnesses}, {"d768-b16-e61-k10"})


if __name__ == "__main__":
    unittest.main()
