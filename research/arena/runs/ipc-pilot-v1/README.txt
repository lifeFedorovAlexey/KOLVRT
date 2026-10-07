IPC functional pilot, schema v1, INELIGIBLE for qualifying Arena admission.

This bundle retains all 96 successful planned functional attempts and the earlier
failed actor execution. Original campaign provenance remains dirty source at
1f6980da7902d5c82f580239cb2984880b7f4958; a later recovered source commit does not
retroactively make the execution clean. Parent will append source recovery identity.

index.json maps original local paths to archive/member plus original SHA-256 and
byte count. Normalize backslashes in raw metadata paths to slash for lookup.
Each distinct ELF digest is stored once; its original_paths lists every alias.
Raw campaign/attempt/build/run/events/log/stderr bytes are unchanged in raw-evidence.zip.
The kernel-build metadata's shared target/kernel/{profile}-native-apps.elf name is
historical: attempt_bindings resolves each such context to its retained exact kernel.
Never resolve that mutable build name to the current local build.

All 646 archived members were decompressed and SHA-256 checked after archive close.
Every archive is smaller than32MiB. ELF files are full original bytes, not stripped,
not extracted code segments, and not relinked. The five definitions reproduce the
successful pilot's recorded raw-byte definition hashes. Initial failure metadata
retains its own original source/definition digests. Preparation files retain the
stack diagnosis, initial failure, startup manifest rejection and actual pilot log.

Run: python research/arena/runs/ipc-pilot-v1/verify.py
This checks archives, every member, and loose definition/supplemental original hashes
without extraction or access to the original machine. It does not execute QEMU,
certify authority/lifetime tests, or promote the pilot into performance acceptance.

Recovery update: successful pilot source is now verified against commit2eac74d59eb82e4a1f17122716eb4647f138cb4a; see source-recovery-review.txt and source-recovery.json. Original dirty execution provenance and failed-actor scope are unchanged.
