"""Small checks for adapter input and correctness boundaries; no Chroma service."""

import tempfile
import unittest
import struct
import zlib
from pathlib import Path

import numpy as np

from chroma_replay import eligible, load_dataset, percentile, read_metadata, validate, where_filter


class ReplayChecks(unittest.TestCase):
    def test_dataset_checksum_length_split_dimension_source_and_vector_validation(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "vectors.qnb"
            config = dict(dimensions="2", rows="2", seed="42")
            payload = struct.pack("<8f", 1, 0, 0, 1, 1, 1, -1, 1)
            header = b"QNLOB001" + struct.pack("<6Q", 2, 2, 1, 1, 42, 0)
            def write(content, expected=None):
                checksum = zlib.crc32(content)
                path.write_bytes(content + struct.pack("<I", checksum if expected is None else expected))
                config["dataset_crc32"] = f"{checksum:08x}"
            write(header + payload)
            splits = load_dataset(path, config)
            self.assertEqual([len(split) for split in splits], [2, 1, 1])
            self.assertEqual(splits[2][0].tolist(), [-1, 1])
            # Release Windows memmap handles before rewriting the test file.
            del splits
            config["dataset_crc32"] = "00000000"
            with self.assertRaises(ValueError):
                load_dataset(path, config)
            for content, footer in [(header + payload, 0), (header + payload[:-1], None),
                                    (b"BADMAGIC" + header[8:] + payload, None),
                                    (b"QNLOB001" + struct.pack("<6Q", 2, 2, 1, 1, 42, 2 << 32) + payload, None),
                                    (header + struct.pack("<8f", 0, 0, 0, 1, 1, 1, -1, 1), None),
                                    (header + struct.pack("<8f", float("nan"), 0, 0, 1, 1, 1, -1, 1), None)]:
                write(content, footer)
                with self.assertRaises(ValueError):
                    load_dataset(path, config)
            write(header + payload)
            config["dimensions"] = "3"
            with self.assertRaises(ValueError):
                load_dataset(path, config)

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
                       dict(ids=[[]], distances=[[]]),
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
