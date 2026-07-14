#!/usr/bin/env python3
"""Interactive runner for the Java/Bedrock cross-edition test matrix.

The runner presents one test at a time, updates the CSV atomically, records the
current Git commit, and snapshots PumpkinRunner's latest.log beside a JSON
record for every result.
"""

from __future__ import annotations

import argparse
import csv
import getpass
import json
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
from collections import Counter
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Iterable, Sequence


STATUS_COLUMN = "Status (Not Tested/Pass/Fail/Blocked/N/A)"
RESULT_COMMANDS = {
    "p": "Pass",
    "pass": "Pass",
    "f": "Fail",
    "fail": "Fail",
    "b": "Blocked",
    "blocked": "Blocked",
    "n": "N/A",
    "na": "N/A",
    "n/a": "N/A",
}
RESULT_STATUSES = ("Not Tested", "Pass", "Fail", "Blocked", "N/A")
GROUP_COLUMNS = ("Run Order", "Test Group", "Group Setup")


@dataclass(frozen=True)
class GroupSpec:
    number: int
    title: str
    setup: str
    test_ids: tuple[str, ...]

    @property
    def label(self) -> str:
        return f"{self.number:02d} - {self.title}"


GROUPS = (
    GroupSpec(
        1,
        "Bootstrap and shared-world baseline",
        "Start a fresh server with a pre-created scoreboard. Join Bedrock, then Java, and keep both clients together at spawn.",
        ("P1-01", "P1-02", "P2-01", "P2-02", "P2-04", "P3-05"),
    ),
    GroupSpec(
        2,
        "Time, weather, and gamerules",
        "Keep both clients online with permission to change time, weather, and supported gamerules.",
        ("P1-07", "P1-08", "P1-09", "P1-10"),
    ),
    GroupSpec(
        3,
        "Inventory, equipment, and basic item exchange",
        "Keep both clients within sight and prepare apples, armor, an offhand item, and several split/mergeable stacks.",
        ("P2-03", "P2-05", "P2-06", "P2-07", "P2-08", "P2-09", "P2-10", "P2-11"),
    ),
    GroupSpec(
        4,
        "Survival vitals and air",
        "Use survival mode near food and deep water; have commands available for max-health and effect changes.",
        ("P2-12", "P2-14", "P2-15", "P2-16", "P2-17", "X-17"),
    ),
    GroupSpec(
        5,
        "Combat, death, and respawn",
        "Keep both clients and the server console visible. Preserve a useful inventory before testing environmental and PvP deaths.",
        ("P2-13", "P2-18", "P2-19", "P2-20", "P2-21", "P2-22"),
    ),
    GroupSpec(
        6,
        "Movement, collision, knockback, and chunks",
        "Use a generated route with varied collision blocks and enough open space for knockback and rapid chunk travel.",
        ("P1-03", "P1-04", "P2-23", "P2-24", "P2-25", "P3-11"),
    ),
    GroupSpec(
        7,
        "Dimensions and recovery replay",
        "Prepare Nether and End portals, non-default player state, and debug logging. Keep Java online as an observer.",
        ("P1-05", "P1-06", "P3-04", "P3-09", "P3-10"),
    ),
    GroupSpec(
        8,
        "Reconnect and session replacement",
        "Move the Bedrock player far from spawn and be ready to disconnect during both normal play and terrain initialization.",
        ("P3-01", "P3-02", "P3-03"),
    ),
    GroupSpec(
        9,
        "Scoreboard updates and recovery",
        "Keep a visible objective and team active while changing scores, decoration, membership, and session boundaries.",
        ("P3-06", "P3-07", "P3-08"),
    ),
    GroupSpec(
        10,
        "Containers and crafting",
        "Prepare a chest, furnace family, crafting table, anvil, smithing table, loom, grindstone, and beacon with valid inputs.",
        ("X-06", "X-07", "X-08", "X-09", "X-10"),
    ),
    GroupSpec(
        11,
        "Complex item components",
        "Prepare damaged, enchanted, named/lore, potion, written-book, map, and trimmed-armor items for two-way transfer.",
        ("X-01", "X-02", "X-03", "X-04", "X-05"),
    ),
    GroupSpec(
        12,
        "Entities, projectiles, vehicles, and fishing",
        "Use an open test area with representative mobs, ranged items, a boat, a minecart, and fishing rods.",
        ("X-11", "X-12", "X-13", "X-14"),
    ),
    GroupSpec(
        13,
        "Social and protocol extras",
        "Keep both clients together with chat/command permissions and Bedrock emotes configured.",
        ("X-15", "X-16"),
    ),
    GroupSpec(
        14,
        "Long-session and future-content coverage",
        "Reserve at least 30 minutes for mixed play; only run the custom-content case when an appropriate server build is available.",
        ("P3-12", "X-18"),
    ),
)


def project_root() -> Path:
    return Path(__file__).resolve().parents[1]


def default_csv_path() -> Path:
    return project_root() / "Bedrock_Cross_Edition_Test_Matrix.csv"


def default_log_path() -> Path:
    configured = os.environ.get("PUMPKIN_LATEST_LOG")
    if configured:
        return Path(configured).expanduser()
    return project_root().parent / "PumpkinRunner" / "logs" / "latest.log"


def default_evidence_path() -> Path:
    return project_root() / "bedrock_test_evidence"


def load_matrix(path: Path) -> tuple[list[dict[str, str]], list[str]]:
    with path.open("r", encoding="utf-8-sig", newline="") as handle:
        reader = csv.DictReader(handle)
        if not reader.fieldnames:
            raise ValueError(f"CSV has no header: {path}")
        rows = [dict(row) for row in reader]
        fieldnames = list(reader.fieldnames)

    required = {"Test ID", STATUS_COLUMN, "Test Procedure", "Expected Result"}
    missing = sorted(required.difference(fieldnames))
    if missing:
        raise ValueError(f"CSV is missing required columns: {', '.join(missing)}")
    return rows, fieldnames


def organized_fieldnames(fieldnames: Sequence[str]) -> list[str]:
    if "Test ID" not in fieldnames:
        raise ValueError("CSV is missing Test ID")
    remainder = [
        field
        for field in fieldnames
        if field not in GROUP_COLUMNS and field != "Test ID"
    ]
    return ["Test ID", *GROUP_COLUMNS, *remainder]


def _group_maps() -> tuple[dict[str, int], dict[str, GroupSpec]]:
    ranks: dict[str, int] = {}
    groups: dict[str, GroupSpec] = {}
    rank = 0
    for group in GROUPS:
        for test_id in group.test_ids:
            if test_id in ranks:
                raise ValueError(f"Test ID appears in multiple groups: {test_id}")
            ranks[test_id] = rank
            groups[test_id] = group
            rank += 1
    return ranks, groups


def organize_rows(
    rows: Sequence[dict[str, str]], fieldnames: Sequence[str]
) -> tuple[list[dict[str, str]], list[str]]:
    ranks, groups = _group_maps()
    seen: set[str] = set()
    organized: list[dict[str, str]] = []

    for original_index, source in enumerate(rows):
        row = dict(source)
        test_id = row.get("Test ID", "").strip().upper()
        if not test_id:
            raise ValueError(f"CSV row {original_index + 2} has no Test ID")
        if test_id in seen:
            raise ValueError(f"Duplicate Test ID: {test_id}")
        seen.add(test_id)
        row["Test ID"] = test_id
        group = groups.get(test_id)
        if group:
            row["Test Group"] = group.label
            row["Group Setup"] = group.setup
        else:
            row["Test Group"] = "99 - Unscheduled"
            row["Group Setup"] = "Review and place this test into a shared setup group."
        row["_original_index"] = str(original_index)
        organized.append(row)

    organized.sort(
        key=lambda row: (
            ranks.get(row["Test ID"], len(ranks) + int(row["_original_index"])),
            row["Test ID"],
        )
    )
    for run_order, row in enumerate(organized, start=1):
        row["Run Order"] = f"{run_order:03d}"
        row.pop("_original_index", None)

    return organized, organized_fieldnames(fieldnames)


def save_matrix(
    path: Path, rows: Sequence[dict[str, str]], fieldnames: Sequence[str]
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            "w",
            encoding="utf-8",
            newline="",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as handle:
            temporary = Path(handle.name)
            writer = csv.DictWriter(
                handle, fieldnames=fieldnames, extrasaction="ignore"
            )
            writer.writeheader()
            writer.writerows(rows)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if temporary and temporary.exists():
            temporary.unlink()


def current_git_commit(repo: Path) -> str:
    process = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"],
        check=False,
        capture_output=True,
        text=True,
    )
    if process.returncode != 0:
        message = process.stderr.strip() or "git rev-parse failed"
        raise RuntimeError(message)
    return process.stdout.strip()


def source_changes(repo: Path, ignored_csv: Path) -> list[str]:
    process = subprocess.run(
        ["git", "-C", str(repo), "status", "--porcelain=v1", "--untracked-files=all"],
        check=False,
        capture_output=True,
        text=True,
    )
    if process.returncode != 0:
        return ["Unable to inspect working tree"]

    try:
        ignored = ignored_csv.resolve().relative_to(repo.resolve()).as_posix()
    except ValueError:
        ignored = ""
    changes = []
    for line in process.stdout.splitlines():
        path_text = line[3:].strip().strip('"').replace("\\", "/")
        if path_text == ignored or path_text.startswith("bedrock_test_evidence/"):
            continue
        changes.append(line)
    return changes


def _safe_component(value: str) -> str:
    safe = "".join(
        character if character.isalnum() or character in "-_" else "_"
        for character in value
    )
    return safe.strip("_") or "unknown"


def _relative_display(path: Path, base: Path) -> str:
    try:
        return path.resolve().relative_to(base.resolve()).as_posix()
    except ValueError:
        return str(path.resolve())


def _append_values(existing: str, values: Iterable[str]) -> str:
    parts = [part.strip() for part in existing.split(";") if part.strip()]
    for value in values:
        value = value.strip()
        if value and value not in parts:
            parts.append(value)
    return "; ".join(parts)


def capture_attempt(
    *,
    evidence_root: Path,
    latest_log: Path,
    test_id: str,
    commit: str,
    completed_at: datetime,
) -> tuple[Path, Path | None, datetime | None, str | None]:
    timestamp = completed_at.strftime("%Y%m%dT%H%M%S_%f%z")
    attempt_dir = (
        evidence_root
        / _safe_component(test_id)
        / f"{timestamp}_{_safe_component(commit[:12])}"
    )
    attempt_dir.mkdir(parents=True, exist_ok=False)

    if not latest_log.is_file():
        return attempt_dir, None, None, f"Server log was not found: {latest_log}"

    source_stat = latest_log.stat()
    destination = attempt_dir / "latest.log"
    temporary = attempt_dir / ".latest.log.tmp"
    try:
        shutil.copy2(latest_log, temporary)
        os.replace(temporary, destination)
    finally:
        if temporary.exists():
            temporary.unlink()
    source_mtime = datetime.fromtimestamp(source_stat.st_mtime).astimezone()
    return attempt_dir, destination, source_mtime, None


def record_result(
    row: dict[str, str],
    *,
    result: str,
    java_observation: str,
    bedrock_observation: str,
    notes: str,
    additional_evidence: str,
    tester: str,
    java_version: str,
    bedrock_version: str,
    commit: str,
    latest_log: Path,
    evidence_root: Path,
    csv_directory: Path,
    completed_at: datetime | None = None,
) -> tuple[Path, str | None]:
    if result not in RESULT_STATUSES[1:]:
        raise ValueError(f"Unsupported result: {result}")
    completed_at = completed_at or datetime.now().astimezone()
    attempt_dir, log_copy, source_mtime, warning = capture_attempt(
        evidence_root=evidence_root,
        latest_log=latest_log,
        test_id=row["Test ID"],
        commit=commit,
        completed_at=completed_at,
    )

    effective_notes = notes.strip()
    if warning:
        effective_notes = _append_values(effective_notes, [warning])

    row[STATUS_COLUMN] = result
    row["Java Observation"] = java_observation.strip()
    row["Bedrock Observation"] = bedrock_observation.strip()
    row["Server Log Timestamp"] = (
        source_mtime.isoformat(timespec="seconds") if source_mtime else "Unavailable"
    )
    row["Tester"] = tester.strip()
    row["Test Date"] = completed_at.date().isoformat()
    row["Server Commit"] = commit
    row["Java Client Version"] = java_version.strip()
    row["Bedrock Client Version"] = bedrock_version.strip()
    row["Notes"] = effective_notes

    evidence_values = [_relative_display(attempt_dir, csv_directory)]
    if additional_evidence.strip():
        evidence_values.append(additional_evidence.strip())
    row["Crash Dump or Evidence Path"] = _append_values(
        row.get("Crash Dump or Evidence Path", ""), evidence_values
    )

    record = {
        "test_id": row["Test ID"],
        "run_order": row.get("Run Order", ""),
        "test_group": row.get("Test Group", ""),
        "result": result,
        "completed_at": completed_at.isoformat(timespec="seconds"),
        "server_commit": commit,
        "tester": tester.strip(),
        "java_client_version": java_version.strip(),
        "bedrock_client_version": bedrock_version.strip(),
        "java_observation": java_observation.strip(),
        "bedrock_observation": bedrock_observation.strip(),
        "notes": effective_notes,
        "additional_evidence": additional_evidence.strip(),
        "latest_log_source": str(latest_log.resolve()),
        "latest_log_copy": str(log_copy.resolve()) if log_copy else None,
        "latest_log_source_mtime": source_mtime.isoformat(timespec="seconds")
        if source_mtime
        else None,
        "snapshot_warning": warning,
    }
    metadata_path = attempt_dir / "result.json"
    metadata_path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    return attempt_dir, warning


def _latest_value(
    rows: Sequence[dict[str, str]], field: str, fallback: str = ""
) -> str:
    for row in reversed(rows):
        value = row.get(field, "").strip()
        if value:
            return value
    return fallback


def _prompt(label: str, default: str = "") -> str:
    suffix = f" [{default}]" if default else ""
    value = input(f"{label}{suffix}: ").strip()
    return value or default


def _format_field(label: str, value: str, width: int) -> str:
    prefix = f"{label}: "
    return textwrap.fill(
        value or "(none)",
        width=width,
        initial_indent=prefix,
        subsequent_indent=" " * len(prefix),
    )


def display_test(
    rows: Sequence[dict[str, str]],
    row: dict[str, str],
    *,
    commit: str,
    latest_log: Path,
    csv_path: Path,
) -> None:
    width = max(80, min(140, shutil.get_terminal_size((110, 30)).columns))
    index = rows.index(row)
    group_rows = [
        candidate
        for candidate in rows
        if candidate.get("Test Group") == row.get("Test Group")
    ]
    group_index = group_rows.index(row) + 1
    statuses = Counter(
        candidate.get(STATUS_COLUMN, "Not Tested") or "Not Tested" for candidate in rows
    )
    rule = "=" * width

    print(f"\n{rule}")
    print(
        f"CSV line {index + 2} | Run {row.get('Run Order', '?')}/{len(rows):03d} | "
        f"Group test {group_index}/{len(group_rows)}"
    )
    print(
        f"Progress: Pass {statuses['Pass']} | Fail {statuses['Fail']} | "
        f"Blocked {statuses['Blocked']} | N/A {statuses['N/A']} | "
        f"Not Tested {statuses['Not Tested'] + statuses['']}"
    )
    print(rule)
    for label, value in (
        ("Test", f"{row['Test ID']} - {row.get('Area', '')}"),
        ("Group", row.get("Test Group", "")),
        ("Shared setup", row.get("Group Setup", "")),
        (
            "Classification",
            f"{row.get('Scope', '')}; priority {row.get('Priority', '')}; "
            f"baseline required {row.get('Required for Baseline', '')}; {row.get('Direction', '')}",
        ),
        ("Preconditions", row.get("Preconditions", "")),
        ("Procedure", row.get("Test Procedure", "")),
        ("Expected", row.get("Expected Result", "")),
        ("Current result", row.get(STATUS_COLUMN, "Not Tested")),
        ("Current evidence", row.get("Crash Dump or Evidence Path", "")),
        ("Server commit", commit),
        ("Live server log", str(latest_log)),
        ("Matrix", str(csv_path)),
    ):
        print(_format_field(label, value, width))
    print(rule)


def print_summary(rows: Sequence[dict[str, str]]) -> None:
    print("\nGrouped progress")
    print("-" * 100)
    for group in GROUPS:
        group_rows = [row for row in rows if row.get("Test Group") == group.label]
        counts = Counter(
            row.get(STATUS_COLUMN, "Not Tested") or "Not Tested" for row in group_rows
        )
        print(
            f"{group.label:<48} total={len(group_rows):>2} pass={counts['Pass']:>2} "
            f"fail={counts['Fail']:>2} blocked={counts['Blocked']:>2} "
            f"not-tested={counts['Not Tested']:>2}"
        )
        tests = "  ".join(
            f"{row['Test ID']}[{row.get(STATUS_COLUMN, 'Not Tested')}]"
            for row in group_rows
        )
        print(f"  {tests}")


def next_test(
    rows: Sequence[dict[str, str]], handled: set[str], include_failures: bool
) -> dict[str, str] | None:
    eligible = {"", "Not Tested"}
    if include_failures:
        eligible.update({"Fail", "Blocked"})
    return next(
        (
            row
            for row in rows
            if row["Test ID"] not in handled
            and row.get(STATUS_COLUMN, "Not Tested") in eligible
        ),
        None,
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run the Bedrock cross-edition CSV test matrix interactively."
    )
    parser.add_argument(
        "--csv", type=Path, default=default_csv_path(), help="test matrix CSV"
    )
    parser.add_argument(
        "--log", type=Path, default=default_log_path(), help="PumpkinRunner latest.log"
    )
    parser.add_argument(
        "--evidence-dir",
        type=Path,
        default=default_evidence_path(),
        help="captured log/result directory",
    )
    parser.add_argument(
        "--repo",
        type=Path,
        default=project_root(),
        help="Git repository for commit capture",
    )
    parser.add_argument(
        "--server-commit", help="override the automatically detected Git HEAD"
    )
    parser.add_argument("--tester", help="tester name (otherwise prompted once)")
    parser.add_argument(
        "--java-version", help="Java client version (otherwise prompted once)"
    )
    parser.add_argument(
        "--bedrock-version", help="Bedrock client version (otherwise prompted once)"
    )
    parser.add_argument(
        "--test-id", help="start at a specific test, including a completed test"
    )
    parser.add_argument(
        "--retest-failed",
        action="store_true",
        help="include Fail and Blocked rows after untested rows",
    )
    parser.add_argument(
        "--one", action="store_true", help="exit after recording or skipping one test"
    )
    parser.add_argument(
        "--list", action="store_true", help="print grouped progress and exit"
    )
    parser.add_argument(
        "--organize-only",
        action="store_true",
        help="organize/update the CSV schema and exit",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    csv_path = args.csv.expanduser().resolve()
    latest_log = args.log.expanduser().resolve()
    evidence_root = args.evidence_dir.expanduser().resolve()
    repo = args.repo.expanduser().resolve()

    try:
        rows, fieldnames = load_matrix(csv_path)
        rows, fieldnames = organize_rows(rows, fieldnames)
        save_matrix(csv_path, rows, fieldnames)
    except (OSError, ValueError) as error:
        print(f"Unable to prepare test matrix: {error}", file=sys.stderr)
        return 2

    if args.organize_only:
        print(f"Organized {len(rows)} tests in {csv_path}")
        return 0
    if args.list:
        print_summary(rows)
        return 0

    try:
        commit = args.server_commit or current_git_commit(repo)
    except RuntimeError as error:
        print(f"Unable to determine server commit: {error}", file=sys.stderr)
        return 2

    changes = source_changes(repo, csv_path)
    if changes:
        print(
            "WARNING: source working tree changes exist; Git HEAD may not describe the running binary:"
        )
        for change in changes[:10]:
            print(f"  {change}")
        if len(changes) > 10:
            print(f"  ... and {len(changes) - 10} more")

    tester = args.tester or _prompt(
        "Tester", _latest_value(rows, "Tester", getpass.getuser())
    )
    java_version = args.java_version or _prompt(
        "Java client version", _latest_value(rows, "Java Client Version")
    )
    bedrock_version = args.bedrock_version or _prompt(
        "Bedrock client version",
        _latest_value(rows, "Bedrock Client Version", "1.26.33"),
    )

    by_id = {row["Test ID"]: row for row in rows}
    forced_id = args.test_id.upper() if args.test_id else None
    if forced_id and forced_id not in by_id:
        print(f"Unknown test ID: {forced_id}", file=sys.stderr)
        return 2

    handled: set[str] = set()
    print(
        "\nCommands: [P]ass [F]ail [B]locked [N]/A [S]kip [J]ump [L]ist [Q]uit [H]elp"
    )
    try:
        while True:
            row = (
                by_id[forced_id]
                if forced_id
                else next_test(rows, handled, args.retest_failed)
            )
            forced_id = None
            if row is None:
                print(
                    "\nNo more selected tests. Use --retest-failed or --test-id ID to revisit results."
                )
                print_summary(rows)
                return 0

            display_test(
                rows, row, commit=commit, latest_log=latest_log, csv_path=csv_path
            )
            command = input("Result/command: ").strip().lower()
            if command in RESULT_COMMANDS:
                result = RESULT_COMMANDS[command]
                java_observation = _prompt(
                    "Java observation", row.get("Java Observation", "")
                )
                bedrock_observation = _prompt(
                    "Bedrock observation", row.get("Bedrock Observation", "")
                )
                notes = _prompt("Notes", row.get("Notes", ""))
                additional_evidence = _prompt(
                    "Additional crash dump/evidence path (optional)"
                )
                attempt_dir, warning = record_result(
                    row,
                    result=result,
                    java_observation=java_observation,
                    bedrock_observation=bedrock_observation,
                    notes=notes,
                    additional_evidence=additional_evidence,
                    tester=tester,
                    java_version=java_version,
                    bedrock_version=bedrock_version,
                    commit=commit,
                    latest_log=latest_log,
                    evidence_root=evidence_root,
                    csv_directory=csv_path.parent,
                )
                save_matrix(csv_path, rows, fieldnames)
                print(f"Saved {row['Test ID']} as {result}; evidence: {attempt_dir}")
                if warning:
                    print(f"WARNING: {warning}")
                handled.add(row["Test ID"])
                if args.one:
                    return 0
            elif command in {"s", "skip"}:
                handled.add(row["Test ID"])
                print(
                    f"Skipped {row['Test ID']} for this run; the CSV was not changed."
                )
                if args.one:
                    return 0
            elif command in {"j", "jump"}:
                target = input("Test ID: ").strip().upper()
                if target in by_id:
                    forced_id = target
                else:
                    print(f"Unknown test ID: {target}")
            elif command in {"l", "list"}:
                print_summary(rows)
                forced_id = row["Test ID"]
            elif command in {"q", "quit", "exit"}:
                return 0
            elif command in {"h", "help", "?"}:
                print(
                    "P/F/B/N records a result and snapshots latest.log. S skips only this session. "
                    "J jumps to any ID. L shows grouped progress. Q saves nothing and exits."
                )
                forced_id = row["Test ID"]
            else:
                print("Unknown command. Enter H for help.")
                forced_id = row["Test ID"]
    except (EOFError, KeyboardInterrupt):
        print("\nStopped. Previously recorded results are already saved.")
        return 130
    except OSError as error:
        print(f"Unable to save test result: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
