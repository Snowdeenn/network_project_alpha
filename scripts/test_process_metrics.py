import os
from pathlib import Path
import tempfile
import time
import unittest
from process_metrics import ProcessMetrics, distribution, summarize_ticks


class MetricsTests(unittest.TestCase):
    def test_current_process_returns_memory_and_cpu(self):
        monitor = ProcessMetrics(os.getpid())
        try:
            first = monitor.sample()
            time.sleep(.02)
            second = monitor.sample()
            self.assertGreater(first["rss_bytes"], 0)
            self.assertGreaterEqual(second["cpu_seconds"], first["cpu_seconds"])
            self.assertIsNotNone(second["cpu_percent_one_core"])
        finally:
            monitor.close()

    def test_percentiles_and_empty_samples(self):
        self.assertEqual(distribution([1, 2, 3, 4])["p50"], 2.5)
        self.assertEqual(distribution([])["count"], 0)
        self.assertIsNone(distribution([])["max"])

    def test_missing_tick_file_is_explicitly_unavailable(self):
        with tempfile.TemporaryDirectory() as directory:
            summary = summarize_ticks(Path(directory) / "missing.jsonl")
            self.assertEqual(summary["samples"], 0)
            self.assertIsNone(summary["timings_ms"]["processing_ms"]["mean"])


if __name__ == "__main__":
    unittest.main()
