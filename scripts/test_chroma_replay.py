"""Small checks for adapter input and correctness boundaries; no Chroma service."""

import tempfile
import unittest
from pathlib import Path

import numpy as np

from chroma_replay import eligible, percentile, read_metadata, validate, where_filter


class ReplayChecks(unittest.TestCase):
    def test_compound_signed_half_open_boundaries_and_empty_truth(self):
        config = dict(filter_user_id="0", filter_timestamp_from="-2", filter_timestamp_to="0")
        self.assertEqual(where_filter(config), {"$and": [{"user_id": {"$eq": 0}},
            {"timestamp_micros": {"$gte": -2}}, {"timestamp_micros": {"$lt": 0}}]})
        self.assertTrue(eligible(dict(user_id=0, timestamp_micros=-2), config))
        self.assertFalse(eligible(dict(user_id=0, timestamp_micros=0), config))
        self.assertFalse(eligible(dict(user_id=1, timestamp_micros=-1), config))
        self.assertEqual(percentile([10, 30, 20, 40], .5), 20)
        corpus = np.array([[1, 0], [0, 1]], dtype=np.float32)
        metadata = [dict(id=0, user_id=0, timestamp_micros=-2), dict(id=1, user_id=1, timestamp_micros=0)]
        self.assertEqual(validate(dict(ids=[["0"]], distances=[[0.0]]), [[1, 0]], corpus, metadata, config, [[0]]), 1)
        self.assertEqual(validate(dict(ids=[[]], distances=[[]]), [[1, 0]], corpus, metadata, config, [[]]), 1)
        for result in [dict(ids=[["0", "0"]], distances=[[0, 0]]),
                       dict(ids=[["1"]], distances=[[1]]),
                       dict(ids=[["0"]], distances=[[.01]]),
                       dict(ids=[["0"]], distances=[[float("nan")]])]:
            with self.assertRaises(ValueError):
                validate(result, [[1, 0]], corpus, metadata, config, [[0]])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "metadata.csv"
            path.write_text("id,user_id,timestamp_micros\n1,0,-2\n")
            with self.assertRaises(ValueError):
                read_metadata(path, 1)


if __name__ == "__main__":
    unittest.main()
