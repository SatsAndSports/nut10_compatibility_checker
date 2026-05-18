#!/usr/bin/env python3

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
TEMPLATE_PATH = ROOT / "_README.md"
OUTPUT_PATH = ROOT / "README.md"
PLACEHOLDER = "{{RESULTS_TABLES}}"
REPORT_SPECS = [
    ("CDK", "cdk", "cdk"),
    ("Nutmix", "nutmix", "nutmix"),
    ("Nutshell", "nutshell", "nutshell"),
    ("Nutshell (Legacy SIG_ALL)", "nutshell-legacySIGALL", "nutshell-legacy-sigall"),
]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def load_report(slug: str) -> dict:
    path = ROOT / "reports" / f"{slug}.json"
    if not path.exists():
        raise FileNotFoundError(f"missing report file: {path}")
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def code(value: str) -> str:
    return f"`{value}`"


def markdown_cell(value: object) -> str:
    text = str(value)
    return text.replace("|", "\\|").replace("\n", " ").strip()


def summary_rows(report: dict) -> list[tuple[str, str]]:
    mint = report.get("mint") or {}
    results = report.get("results") or []
    passed = sum(1 for result in results if result.get("status") == "pass")
    failed = sum(1 for result in results if result.get("status") == "fail")
    rows = [
        ("Version", code(mint.get("version") or "<unknown>")),
        ("Mint URL", code(report.get("mint_url") or "<unknown>")),
    ]

    mint_name = (mint.get("name") or "").strip()
    if mint_name:
        rows.append(("Mint Name", markdown_cell(mint_name)))

    if report.get("sigall_mode") == "legacy":
        rows.append(("SIG_ALL Mode", code("legacy")))

    rows.extend(
        [
            ("Started At", code(report.get("generated_at_utc") or "<unknown>")),
            ("Attempted", str(len(results))),
            ("Passed", f"{passed} ✅"),
            ("Failed", f"{failed} {'❌' if failed else '✅'}"),
        ]
    )
    return rows


def render_summary_table(report: dict) -> str:
    rows = ["| Field | Value |", "|---|---|"]
    for field, value in summary_rows(report):
        rows.append(f"| {markdown_cell(field)} | {value} |")
    return "\n".join(rows)


def render_results_table(report: dict) -> str:
    rows = ["| Scenario | Result | Note |", "|---|---|---|"]
    for result in report.get("results") or []:
        status = result.get("status")
        icon = "✅" if status == "pass" else "❌"
        rows.append(
            "| {scenario} | {icon} | {note} |".format(
                scenario=code(markdown_cell(result.get("name") or "<unknown>")),
                icon=icon,
                note=markdown_cell(result.get("note") or ""),
            )
        )
    return "\n".join(rows)


def render_section(title: str, slug: str, anchor: str) -> str:
    report = load_report(slug)
    results = report.get("results") or []
    failed = sum(1 for result in results if result.get("status") == "fail")
    summary = render_summary_table(report)
    details_summary = (
        f"Scenario Results ({len(results)} scenarios, {failed} failure(s))"
    )
    results_table = render_results_table(report)
    return "\n".join(
        [
            f'<a id="{anchor}"></a>',
            "",
            f"### {title}",
            "",
            summary,
            "",
            "<details>",
            f"<summary>{details_summary}</summary>",
            "",
            results_table,
            "",
            "</details>",
        ]
    )


def render_results_sections() -> str:
    lines = [
        "## Current Results",
        "",
        "> Generated from `_README.md` and `reports/*.json` via `python3 tools/build_readme.py`.",
        "",
        "Jump to:",
        "",
    ]
    for title, _slug, anchor in REPORT_SPECS:
        lines.append(f"- [{title}](#{anchor})")
    lines.append("")

    for index, (title, slug, anchor) in enumerate(REPORT_SPECS):
        lines.append(render_section(title, slug, anchor))
        if index != len(REPORT_SPECS) - 1:
            lines.append("")

    return "\n".join(lines)


def main() -> None:
    template = read_text(TEMPLATE_PATH)
    if PLACEHOLDER not in template:
        raise ValueError(f"missing placeholder {PLACEHOLDER!r} in {TEMPLATE_PATH}")

    readme = template.replace(PLACEHOLDER, render_results_sections())
    OUTPUT_PATH.write_text(readme, encoding="utf-8")


if __name__ == "__main__":
    main()
