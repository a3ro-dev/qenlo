"""Temporary-artifact acceptance checks; standard library only."""

import csv
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from compare_runs import compare, lower_median, read_run


class ComparisonChecks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def fixture(self, name, latencies=(100, 100, 100, 100, 100), chroma=False,
                changes=None, recalls=None, tuning=1.0, declared=True):
        path = self.root / name
        path.mkdir()
        config = dict(dataset_crc32="abcd0123", dimensions="32", rows="100", eligible_count="10",
                      batch="1", metadata="synthetic-independent", filter_user_id="0",
                      filter_timestamp_from="-10", filter_timestamp_to="0", filter_mode="shared",
                      corpus_range="0..100", tuning_range="100..110", evaluation_range="110..130",
                      seed="42", k="10", warmup_queries="2", repetitions=str(len(latencies)),
                      recall_target="0.95", backend="chroma" if chroma else "cpu", platform="test")
        config.update(changes or {})
        recalls = recalls or [1.0] * len(latencies)
        summary = dict(status="completed", tuning_recall_at_10=tuning,
                       evaluation_recall_at_10=sum(recalls) / len(recalls), filter_violations=0,
                       median_run_p95_batch_ns=lower_median(latencies), recall_target_passed=declared)
        for name, data in [("configuration", config), ("summary", summary)]:
            suffix = ".json" if chroma else ".txt"
            (path / (name + suffix)).write_text(json.dumps(data) if chroma else
                "\n".join(f"{key}={value}" for key, value in data.items()), encoding="utf-8")
        with (path / "runs.csv").open("w", newline="") as stream:
            writer = csv.writer(stream)
            writer.writerow(["run", "queries", "batches", "p95_batch_ns", "recall_at_10"])
            for index, (latency, recall) in enumerate(zip(latencies, recalls)):
                writer.writerow([index, 20, 20, latency, recall])
        return path

    def test_constant_ratio_bootstrap_and_api_boundaries(self):
        result = compare(self.fixture("cpu"), self.fixture("chroma", (50,) * 5, chroma=True))
        self.assertTrue(result["latency_comparison_valid"])
        self.assertEqual(result["baseline_over_candidate_median_p95_ratio"], 2)
        self.assertEqual(result["bootstrap_95_percent_interval"], [2, 2])
        self.assertEqual(result["bootstrap"]["draws"], 10000)
        self.assertTrue(result["api_boundaries_differ"])
        self.assertEqual(len(result["candidate"]["runs"]), 5)

    def test_lower_middle_and_seeded_interval(self):
        base = self.fixture("base", (100, 200, 300, 400))
        other = self.fixture("other", (40, 50, 60, 70))
        result = compare(base, other, seed=71)
        self.assertEqual(result["baseline"]["median_run_p95_batch_ns"], 200)
        self.assertEqual(result["baseline_over_candidate_median_p95_ratio"], 4)
        self.assertEqual(result, compare(base, other, seed=71))
        low, high = result["bootstrap_95_percent_interval"]
        self.assertLessEqual(low, 4)
        self.assertGreaterEqual(high, 4)

    def test_each_workload_mismatch_is_rejected(self):
        baseline = self.fixture("baseline")
        for index, (key, value) in enumerate({
            "dataset_crc32": "abcd0124", "dimensions": "64", "rows": "101",
            "eligible_count": "9", "batch": "2", "metadata": "synthetic-skewed",
            "filter_user_id": "1", "filter_timestamp_from": "-9", "filter_timestamp_to": "1",
            "evaluation_range": "111..131", "tuning_range": "101..111",
        }.items()):
            with self.subTest(key=key), self.assertRaises(ValueError):
                compare(baseline, self.fixture(f"other-{index}", changes={key: value}))

    def test_failed_tuning_or_one_failed_run_withholds_ratio(self):
        baseline = self.fixture("baseline")
        candidates = [self.fixture("tuning", tuning=.9),
                      self.fixture("one-bad-run", recalls=[.94, 1, 1, 1, 1]),
                      self.fixture("declared-failure", declared=False)]
        for candidate in candidates:
            with self.subTest(candidate=candidate.name):
                result = compare(baseline, candidate)
                self.assertFalse(result["latency_comparison_valid"])
                self.assertIsNone(result["baseline_over_candidate_median_p95_ratio"])
                self.assertIsNone(result["bootstrap_95_percent_interval"])

    def test_recall_gate_tolerates_accumulation_roundoff_without_rounding_values(self):
        recall = .9899999999999999
        path = self.fixture("rounded-sum", changes={"recall_target": ".99"},
                            recalls=[recall] * 5, tuning=recall)
        result = read_run(path)
        self.assertTrue(result["recall_gate"]["passed"])
        self.assertEqual(result["tuning_recall_at_10"], recall)
        self.assertEqual(result["runs"][0]["recall_at_10"], recall)
        below = self.fixture("below-threshold", changes={"recall_target": ".99"},
                             recalls=[.99 - 1e-10] * 5)
        self.assertFalse(read_run(below)["recall_gate"]["passed"])

    def test_incomplete_corrupt_and_nonfinite_artifacts_fail(self):
        for index, changes in enumerate([{"status": "incomplete"}, {"tuning_recall_at_10": "NaN"},
                                         {"median_run_p95_batch_ns": "99"}, {"filter_violations": "1"}]):
            path = self.fixture(f"bad-{index}")
            summary = path / "summary.txt"
            data = dict(line.split("=", 1) for line in summary.read_text().splitlines())
            data.update(changes)
            summary.write_text("\n".join(f"{key}={value}" for key, value in data.items()))
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                read_run(path)
        path = self.fixture("bad-runs")
        (path / "runs.csv").write_text("run,queries,batches,p95_batch_ns,recall_at_10\n0,20,20,0,1\n")
        with self.assertRaises(ValueError):
            read_run(path)

    def test_run_table_rejects_duplicate_ids_zero_latency_and_wrong_query_count(self):
        for index, (field, value) in enumerate([("run", "1"), ("p95_batch_ns", "0"),
                                               ("queries", "19"), ("recall_at_10", "inf")]):
            path = self.fixture(f"invalid-table-{index}")
            with (path / "runs.csv").open(newline="") as stream:
                rows = list(csv.DictReader(stream))
            rows[0][field] = value
            with (path / "runs.csv").open("w", newline="") as stream:
                writer = csv.DictWriter(stream, fieldnames=rows[0].keys())
                writer.writeheader()
                writer.writerows(rows)
            with self.subTest(field=field), self.assertRaises(ValueError):
                read_run(path)

    def test_cli_writes_json_and_never_overwrites(self):
        baseline, candidate = self.fixture("base"), self.fixture("candidate")
        output = self.root / "comparison.json"
        command = [sys.executable, str(Path(__file__).with_name("compare_runs.py")), "--baseline", str(baseline),
                   "--candidate", str(candidate), "--output", str(output)]
        first = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(first.returncode, 0, first.stderr)
        original = output.read_bytes()
        self.assertEqual(json.loads(original)["format"], "qenlo-run-comparison-v1")
        second = subprocess.run(command, capture_output=True, text=True)
        self.assertNotEqual(second.returncode, 0)
        self.assertEqual(output.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
