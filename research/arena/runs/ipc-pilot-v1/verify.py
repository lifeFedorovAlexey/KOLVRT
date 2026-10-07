"""Verify original evidence bytes without extraction or external dependencies."""
from pathlib import Path
import hashlib
import json
import zipfile

root = Path(__file__).resolve().parent
index = json.loads((root / "index.json").read_text(encoding="utf-8"))
assert index["admission_state"] == "INELIGIBLE" and index["record_eligible"] is False

def digest(data):
    return hashlib.sha256(data).hexdigest()

count = 0
for archive in index["archives"]:
    path = root / archive["path"]
    data = path.read_bytes()
    assert len(data) == archive["bytes"] < 32 * 1024 * 1024
    assert digest(data) == archive["sha256"]
    entries = [item for item in index["files"] if item["archive"] == archive["path"]]
    with zipfile.ZipFile(path) as handle:
        assert len(handle.namelist()) == len(entries) == archive["members"]
        assert set(handle.namelist()) == {item["member"] for item in entries}
        for item in entries:
            data = handle.read(item["member"])
            assert len(data) == item["bytes"] and digest(data) == item["sha256"]
            if item["type"] == "original-elf":
                assert data[:4] == b"\x7fELF"
            count += 1
for section in ("definitions", "supplemental"):
    for item in index[section]:
        data = (root / item["path"]).read_bytes()
        assert len(data) == item["bytes"] and digest(data) == item["sha256"]
assert count == index["verification"]["verified_members"]
print(f"Verified {count} original archive members plus definitions/supplemental bytes; pilot remains INELIGIBLE")
