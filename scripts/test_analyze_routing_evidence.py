import csv
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


if __name__ == "__main__":
    unittest.main()
