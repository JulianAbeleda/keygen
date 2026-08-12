#!/usr/bin/env python3
"""Validate the kg_ddlc_plus execution ledger.

The ledger is intentionally Markdown so it remains readable to operators.  This
small, dependency-free parser makes the parts agents rely on machine-checkable:
packet identity, dependency expansion, status vocabulary, report references,
and acyclic ordering.  It does not interpret deliverable prose.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ID_RE = re.compile(r"\bKGD-(\d{3})\b")
HEADING_RE = re.compile(r"^###\s+(KGD-\d{3})\b")
TABLE_RE = re.compile(r"^\|\s*(KGD-\d{3})\s*\|")
STATUS_RE = re.compile(r"\bStatus:\s*([A-Za-z_-]+)", re.IGNORECASE)
REPORT_RE = re.compile(r"(?:docs/)?task_reports/(KGD-\d{3})\.md")
RANGE_RE = re.compile(r"^(\d{3})\s*[–—-]\s*(\d{3})$")

LEGAL_STATUSES = {"planned", "in_progress", "blocked", "complete", "deferred"}


@dataclass
class Packet:
    packet_id: str
    line: int
    dependencies: list[str]
    status: str | None = None


def packet_id(number: str) -> str:
    return f"KGD-{int(number):03d}"


def expand_dependencies(cell: str) -> list[str]:
    """Expand the compact ``120–132`` notation used by the ledger."""
    if not cell.strip() or cell.strip() in {"—", "-", "none", "None"}:
        return []
    result: list[str] = []
    for token in re.split(r"\s*,\s*", cell.strip()):
        token = token.strip().strip("`")
        match = RANGE_RE.fullmatch(token)
        if match:
            start, end = (int(value) for value in match.groups())
            if end < start:
                raise ValueError(f"descending dependency range {token!r}")
            result.extend(packet_id(str(value)) for value in range(start, end + 1))
            continue
        if re.fullmatch(r"\d{3}", token):
            result.append(packet_id(token))
            continue
        # Keep the diagnostic local to this packet rather than silently
        # accepting a typo such as "120-".
        raise ValueError(f"invalid dependency token {token!r}")
    return result


def parse_ledger(path: Path) -> tuple[list[Packet], list[str]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    packets: list[Packet] = []
    errors: list[str] = []
    for index, line in enumerate(lines, start=1):
        heading = HEADING_RE.match(line)
        table = TABLE_RE.match(line)
        if heading:
            packets.append(Packet(heading.group(1), index, []))
            continue
        if not table:
            continue
        # Markdown table columns are packet ID, deliverable, dependencies,
        # acceptance. Splitting once per column keeps prose pipes harmless.
        columns = [column.strip() for column in line.strip().strip("|").split("|")]
        if len(columns) < 3:
            errors.append(f"line {index}: packet row has fewer than 3 columns")
            continue
        try:
            dependencies = expand_dependencies(columns[2])
        except ValueError as exc:
            errors.append(f"line {index}: {exc}")
            dependencies = []
        packets.append(Packet(table.group(1), index, dependencies))

    # A status marker applies to the nearest preceding packet heading.  This
    # supports both the compact table rows and the detailed KGD-000 section.
    current: Packet | None = None
    for index, line in enumerate(lines, start=1):
        heading = HEADING_RE.match(line)
        if heading:
            current = next((p for p in reversed(packets) if p.packet_id == heading.group(1) and p.line == index), None)
        status = STATUS_RE.search(line)
        if status and current:
            current.status = status.group(1).lower().replace("-", "_")
    return packets, errors


def find_cycles(graph: dict[str, list[str]]) -> list[list[str]]:
    colours: dict[str, int] = {}
    stack: list[str] = []
    cycles: list[list[str]] = []

    def visit(node: str) -> None:
        colours[node] = 1
        stack.append(node)
        for dependency in graph.get(node, []):
            if dependency not in graph:
                continue
            if colours.get(dependency, 0) == 0:
                visit(dependency)
            elif colours.get(dependency) == 1:
                start = stack.index(dependency)
                cycles.append(stack[start:] + [dependency])
        stack.pop()
        colours[node] = 2

    for node in graph:
        if colours.get(node, 0) == 0:
            visit(node)
    return cycles


def validate(path: Path) -> dict[str, object]:
    packets, errors = parse_ledger(path)
    occurrences: dict[str, list[int]] = {}
    for packet in packets:
        occurrences.setdefault(packet.packet_id, []).append(packet.line)
        if packet.status and packet.status not in LEGAL_STATUSES:
            errors.append(f"line {packet.line}: illegal status {packet.status!r} for {packet.packet_id}")
    duplicates = {key: lines for key, lines in occurrences.items() if len(lines) > 1}
    errors.extend(f"duplicate packet {key} at lines {lines}" for key, lines in duplicates.items())

    known = set(occurrences)
    graph = {packet.packet_id: packet.dependencies for packet in packets}
    missing = sorted({dependency for deps in graph.values() for dependency in deps if dependency not in known})
    errors.extend(f"missing dependency {dependency}" for dependency in missing)
    cycles = find_cycles(graph)
    errors.extend("dependency cycle: " + " -> ".join(cycle) for cycle in cycles)

    text = path.read_text(encoding="utf-8")
    report_refs = sorted(set(REPORT_RE.findall(text)))
    unknown_reports = sorted(set(report_refs) - known)
    errors.extend(f"report references unknown packet {report}" for report in unknown_reports)

    # A report link is checked when the ledger explicitly declares one.  This
    # lets the ledger describe planned work without requiring empty report
    # files, while still rejecting stale or misspelled links.
    reports_dir = path.parent / "task_reports"
    absent_reports = [report for report in report_refs if not (reports_dir / f"{report}.md").exists()]
    errors.extend(f"missing referenced report {report}" for report in absent_reports)

    return {
        "ledger": str(path),
        "packet_count": len(packets),
        "packets": [
            {"id": p.packet_id, "line": p.line, "depends": p.dependencies, "status": p.status}
            for p in packets
        ],
        "duplicate_ids": duplicates,
        "missing_dependencies": missing,
        "cycles": cycles,
        "report_references": report_refs,
        "errors": errors,
        "ok": not errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ledger", nargs="?", type=Path, default=Path("docs/kg_ddlc_plus_tasks.md"))
    parser.add_argument("--json", action="store_true", dest="as_json", help="emit the machine-readable result")
    args = parser.parse_args()
    result = validate(args.ledger)
    if args.as_json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(f"{result['ledger']}: {result['packet_count']} packets")
        if result["errors"]:
            for error in result["errors"]:
                print(f"error: {error}", file=sys.stderr)
        else:
            print("ok: unique IDs, dependencies, statuses, report references, and no cycles")
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
