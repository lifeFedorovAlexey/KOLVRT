"""Render deterministic review tables from validated records."""
import argparse
from collections import Counter
from pathlib import Path
import sys

from validate import ROOT, read_json, validate_database


def render():
    cases = [read_json(p) for p in sorted((ROOT / "research/pathology").glob("*.json"))]
    lines = ["# Pathology cases — Phase 0.1", "", "Дата исследования: 2026-10-02. "
             "Решения KOLVRT — проектные выводы, не реализованное поведение.", "",
             "| ID | Случай | Подсистема | Решение | Compat | Уверенность |",
             "|---|---|---|---|---|---|"]
    for c in cases:
        lines.append(f"| [{c['id']}](../../research/pathology/{c['id']}.json) | "
                     f"{c['title']} | {c['subsystem']} | {c['kolvrt_decision']} | "
                     f"{c['compatibility_required']} | {c['confidence']} |")
    lines += ["", "## Группировка решений", ""]
    counts = Counter(c["kolvrt_decision"] for c in cases)
    for decision in ("NATIVE_FIX", "COMPAT_ONLY", "HARDWARE_TRANSLATION", "ACCEPTED_TRADEOFF",
                     "RESEARCH_REQUIRED", "NOT_APPLICABLE"):
        ids = ", ".join(c["id"] for c in cases if c["kolvrt_decision"] == decision) or "нет"
        lines.append(f"- **{decision} ({counts[decision]})**: {ids}.")
    lines += ["", "## Вопросы по записям", ""]
    for c in cases:
        if c["open_questions"]:
            lines.append(f"- **{c['id']}**: " + "; ".join(c["open_questions"]))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    errors = validate_database(ROOT / "research/pathology", ROOT / "schemas/pathology/case.schema.json", 30)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    target = ROOT / "docs/research/CASE_INDEX.md"
    result = render()
    if args.check:
        if not target.exists() or target.read_text(encoding="utf-8") != result:
            print("CASE_INDEX.md is stale", file=sys.stderr)
            return 1
        print("Case index is current.")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(result, encoding="utf-8")
        print(target)
    return 0


if __name__ == "__main__":
    sys.exit(main())
