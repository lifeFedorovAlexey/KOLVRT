"""Offline structural and cross-record validation; never claims historical truth."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from urllib.parse import urlparse

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]
FACT_FIELDS = (
    "historical_context", "original_reason", "root_cause", "observable_behavior",
    "compatibility_dependency", "linux_current_solution",
)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def read_json(path):
    def invalid_constant(value):
        raise ValueError(f"non-finite JSON number: {value}")
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object,
                      parse_constant=invalid_constant)


def validate_database(directory, schema_path, minimum=1):
    errors = []
    try:
        schema = read_json(schema_path)
        Draft202012Validator.check_schema(schema)
    except (OSError, ValueError) as exc:
        return [f"schema: {exc}"]
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    files = sorted(directory.glob("*.json"))
    if len(files) < minimum:
        errors.append(f"expected at least {minimum} case files, found {len(files)}")
    seen = set()
    for path in files:
        try:
            case = read_json(path)
        except (OSError, ValueError) as exc:
            errors.append(f"{path.name}: {exc}")
            continue
        structural = sorted(validator.iter_errors(case), key=lambda e: str(list(e.path)))
        if structural:
            errors.extend(f"{path.name}:{'/'.join(map(str, e.path))}: {e.message}"
                          for e in structural)
            continue
        cid = case["id"]
        if cid in seen:
            errors.append(f"{path.name}: duplicate ID {cid}")
        seen.add(cid)
        if path.stem != cid:
            errors.append(f"{path.name}: filename must match {cid}")
        source_ids = [s["id"] for s in case["sources"]]
        if len(set(source_ids)) != len(source_ids):
            errors.append(f"{cid}: duplicate source ID")
        for source in case["sources"]:
            parsed = urlparse(source["url"])
            if parsed.scheme not in ("http", "https") or not parsed.netloc:
                errors.append(f"{cid}: invalid HTTP source URL")
            if source["research_date"] != case["research_date"]:
                errors.append(f"{cid}: source research date differs from case")
            if source["date"] and source["date"] > source["research_date"]:
                errors.append(f"{cid}: source date is after research date")
            if any(source[k] is None for k in ("commit", "tag_version", "date")):
                if not source["provenance_limitations"].strip():
                    errors.append(f"{cid}: unknown provenance needs an explanation")
        for field in FACT_FIELDS:
            refs = case["evidence"].get(field, [])
            if not refs or any(ref not in source_ids for ref in refs):
                errors.append(f"{cid}: invalid or missing evidence for {field}")
        if case["first_known_version"] is None and not case["open_questions"]:
            errors.append(f"{cid}: unknown first version needs an open question")
        decision = case["kolvrt_decision"]
        compat = case["compatibility_required"]
        if decision == "COMPAT_ONLY" and compat != "YES":
            errors.append(f"{cid}: COMPAT_ONLY requires YES")
        if compat == "NO" and case["compatibility_scope"] != "NONE":
            errors.append(f"{cid}: NO compatibility requires NONE scope")
        if compat in ("YES", "CONDITIONAL") and case["compatibility_scope"] == "NONE":
            errors.append(f"{cid}: compatibility requires a concrete scope")
        if decision == "RESEARCH_REQUIRED" and not case["open_questions"]:
            errors.append(f"{cid}: research decision requires open questions")
        if case["status"] == "UNRESOLVED" and not case["open_questions"]:
            errors.append(f"{cid}: unresolved case requires open questions")
        if not any(s["kind"] in ("SOURCE_CODE", "COMMIT", "OFFICIAL_DOCUMENTATION",
                                  "BUG_REPORT", "REPRODUCER", "MAINTAINER_DISCUSSION")
                   for s in case["sources"]):
            errors.append(f"{cid}: at least one primary source required")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=ROOT / "research/pathology")
    parser.add_argument("--schema", type=Path, default=ROOT / "schemas/pathology/case.schema.json")
    parser.add_argument("--minimum", type=int, default=30)
    args = parser.parse_args()
    if args.minimum < 1:
        parser.error("--minimum must be positive")
    try:
        errors = validate_database(args.directory, args.schema, args.minimum)
    except Exception as exc:
        print(f"validation could not complete: {exc}", file=sys.stderr)
        return 2
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print(f"Validated {len(list(args.directory.glob('*.json')))} cases: schema and consistency OK. "
          "Historical claims and kernel behavior are not execution-tested.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
