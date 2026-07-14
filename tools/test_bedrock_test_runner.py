from __future__ import annotations

import csv
import json
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path

from tools import bedrock_test_runner as runner


class BedrockTestRunnerTests(unittest.TestCase):
    def test_group_plan_contains_65_unique_tests(self) -> None:
        test_ids = [test_id for group in runner.GROUPS for test_id in group.test_ids]
        self.assertEqual(65, len(test_ids))
        self.assertEqual(65, len(set(test_ids)))

    def test_group_plan_exactly_covers_repository_matrix(self) -> None:
        rows, _ = runner.load_matrix(runner.default_csv_path())
        matrix_ids = {row["Test ID"] for row in rows}
        grouped_ids = {test_id for group in runner.GROUPS for test_id in group.test_ids}

        self.assertEqual(matrix_ids, grouped_ids)
        self.assertEqual(
            [f"{position:03d}" for position in range(1, 66)],
            [row["Run Order"] for row in rows],
        )

    def test_organize_rows_groups_and_orders_known_tests(self) -> None:
        fields = ["Test ID", runner.STATUS_COLUMN, "Test Procedure", "Expected Result"]
        rows = [
            {"Test ID": "X-18", runner.STATUS_COLUMN: "Not Tested"},
            {"Test ID": "P1-07", runner.STATUS_COLUMN: "Pass"},
            {"Test ID": "P1-01", runner.STATUS_COLUMN: "Not Tested"},
        ]

        organized, output_fields = runner.organize_rows(rows, fields)

        self.assertEqual(
            ["P1-01", "P1-07", "X-18"], [row["Test ID"] for row in organized]
        )
        self.assertEqual(["001", "002", "003"], [row["Run Order"] for row in organized])
        self.assertEqual(
            "01 - Bootstrap and shared-world baseline", organized[0]["Test Group"]
        )
        self.assertEqual(["Test ID", *runner.GROUP_COLUMNS], output_fields[:4])
        self.assertEqual("Pass", organized[1][runner.STATUS_COLUMN])

    def test_save_and_load_matrix_round_trip_quoted_text(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "matrix.csv"
            fields = [
                "Test ID",
                runner.STATUS_COLUMN,
                "Test Procedure",
                "Expected Result",
            ]
            rows = [
                {
                    "Test ID": "T-01",
                    runner.STATUS_COLUMN: "Not Tested",
                    "Test Procedure": "Move, split, and merge",
                    "Expected Result": 'Displays "apple"',
                }
            ]
            runner.save_matrix(path, rows, fields)
            loaded, loaded_fields = runner.load_matrix(path)

            self.assertEqual(fields, loaded_fields)
            self.assertEqual(rows, loaded)
            with path.open("r", encoding="utf-8", newline="") as handle:
                self.assertEqual(1, len(list(csv.DictReader(handle))))

    def test_record_result_copies_log_and_writes_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            latest_log = root / "PumpkinRunner" / "logs" / "latest.log"
            latest_log.parent.mkdir(parents=True)
            latest_log.write_text("server log line\n", encoding="utf-8")
            evidence = root / "evidence"
            row = {
                "Test ID": "P1-01",
                "Run Order": "001",
                "Test Group": "01 - Bootstrap",
                runner.STATUS_COLUMN: "Not Tested",
                "Crash Dump or Evidence Path": "",
            }
            completed = datetime(2026, 7, 14, 12, 30, tzinfo=timezone.utc)

            attempt_dir, warning = runner.record_result(
                row,
                result="Fail",
                java_observation="Java stayed connected",
                bedrock_observation="Bedrock disconnected",
                notes="Reproduced once",
                additional_evidence="C:/MinecraftCrashDumps/example.dmp",
                tester="Tester",
                java_version="1.21.8",
                bedrock_version="1.26.33",
                commit="0123456789abcdef",
                latest_log=latest_log,
                evidence_root=evidence,
                csv_directory=root,
                completed_at=completed,
            )

            self.assertIsNone(warning)
            self.assertEqual("Fail", row[runner.STATUS_COLUMN])
            self.assertEqual("0123456789abcdef", row["Server Commit"])
            self.assertIn("evidence/P1-01/", row["Crash Dump or Evidence Path"])
            self.assertIn(
                "C:/MinecraftCrashDumps/example.dmp", row["Crash Dump or Evidence Path"]
            )
            self.assertEqual(
                "server log line\n",
                (attempt_dir / "latest.log").read_text(encoding="utf-8"),
            )
            metadata = json.loads(
                (attempt_dir / "result.json").read_text(encoding="utf-8")
            )
            self.assertEqual("P1-01", metadata["test_id"])
            self.assertEqual("Fail", metadata["result"])
            self.assertEqual(str(latest_log.resolve()), metadata["latest_log_source"])

    def test_missing_log_is_recorded_without_losing_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            row = {
                "Test ID": "P1-02",
                "Run Order": "002",
                "Test Group": "01 - Bootstrap",
                runner.STATUS_COLUMN: "Not Tested",
                "Crash Dump or Evidence Path": "",
            }
            attempt_dir, warning = runner.record_result(
                row,
                result="Blocked",
                java_observation="",
                bedrock_observation="",
                notes="",
                additional_evidence="",
                tester="Tester",
                java_version="",
                bedrock_version="1.26.33",
                commit="abcdef",
                latest_log=root / "missing.log",
                evidence_root=root / "evidence",
                csv_directory=root,
                completed_at=datetime(2026, 7, 14, tzinfo=timezone.utc),
            )

            self.assertIsNotNone(warning)
            self.assertEqual("Blocked", row[runner.STATUS_COLUMN])
            self.assertEqual("Unavailable", row["Server Log Timestamp"])
            self.assertTrue((attempt_dir / "result.json").is_file())
            self.assertIn("Server log was not found", row["Notes"])


if __name__ == "__main__":
    unittest.main()
