# DKV M3 Reconciliation — Storage Contract Compliance

## Mission

Reconcile the current DKV implementation against the **canonical Phase 3 contract**:

```text
specs/M3-storage.md
```

That file is the **authoritative source of truth** for M3.

Do not use previous chat prompts, assumptions, or memory as the specification when they conflict with `specs/M3-storage.md`.

---

## Current State

DKV is an offline-first Rust CLI.

Completed:

* M1 — Project Understanding
* M2 — Knowledge Acquisition Foundation

Current storage implementation already exists.

Do **not** rewrite the storage subsystem from scratch.

Your job is to:

```text
CURRENT IMPLEMENTATION
        ↓
COMPARE
        ↓
specs/M3-storage.md
        ↓
IDENTIFY DEVIATIONS
        ↓
FIX ONLY REQUIRED GAPS
        ↓
TEST
        ↓
VERIFY
```

---

# Step 1 — Read the Contract First

Before modifying code:

```bash
cat specs/M3-storage.md
```

Then inspect the actual implementation:

```bash
find src/storage -type f -maxdepth 2 -print
```

Inspect related modules:

```bash
find src/providers src/project src/scanner src/types -type f -print
```

Inspect CLI wiring:

```bash
sed -n '1,260p' src/cli.rs
```

Inspect dependencies:

```bash
cat Cargo.toml
```

Do not make changes until you understand the existing architecture.

---

# Step 2 — Establish Baseline

Run:

```bash
cargo fmt --check
cargo check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test
cargo doc --no-deps
```

Record the baseline.

If something already fails, distinguish:

```text
PRE-EXISTING FAILURE
```

from:

```text
M3 RECONCILIATION FAILURE
```

Do not hide failures.

---

# Step 3 — Build a Contract Matrix

Create a temporary working checklist mapping every M3 acceptance criterion to the implementation.

Use this format:

```text
M3 Criterion                         Implementation        Status
-------------------------------------------------------------------
Vault layout                         ...                   PASS/FAIL
SQLite backend                       ...                   PASS/FAIL
Migrations                           ...                   PASS/FAIL
Package identity                     ...                   PASS/FAIL
NULL-version deduplication           ...                   PASS/FAIL
Source identity                      ...                   PASS/FAIL
Document identity                    ...                   PASS/FAIL
Idempotent ingestion                 ...                   PASS/FAIL
Checksum storage                     ...                   PASS/FAIL
Content deduplication                ...                   PASS/FAIL
Metadata persistence                 ...                   PASS/FAIL
Filesystem paths                     ...                   PASS/FAIL
Transactional ingestion              ...                   PASS/FAIL
Rollback cleanup                     ...                   PASS/FAIL
Ingest runs                          ...                   PASS/FAIL
dkv ingest                           ...                   PASS/FAIL
dkv vault status                     ...                   PASS/FAIL
dkv vault verify                     ...                   PASS/FAIL
Storage statistics                   ...                   PASS/FAIL
Tests                                ...                   PASS/FAIL
Five quality gates                   ...                   PASS/FAIL
```

Use the actual contract for the complete checklist.

---

# Step 4 — Reconcile the Existing Implementation

Fix every genuine contract violation.

### Important

Do not blindly implement hypothetical functionality.

Only change code when:

1. the contract requires it;
2. the existing implementation violates it; or
3. a test is required to prove an acceptance criterion.

---

# Step 5 — Preserve Existing Architecture

Do NOT:

* rewrite scanner
* rewrite providers
* rename existing project modules
* move unrelated files
* redesign the CLI
* introduce an unnecessary framework
* replace working implementations merely for stylistic reasons
* add search
* add indexing
* add network providers
* add AI/RAG

Preserve the current structure.

---

# Step 6 — Storage Correctness

Pay particular attention to the contract's requirements for:

### Package identity

Packages without versions must deduplicate correctly.

Do not accidentally rely on SQL NULL uniqueness semantics.

### Document identity

Repeated ingestion of:

```text
package + source + relative_path
```

must not create duplicate logical documents.

### Content deduplication

Identical content must not create unnecessary physical copies.

### Checksums

Checksums must represent actual content.

### Filesystem paths

Persist the correct source filesystem path and vault content path.

### Idempotency

Running:

```bash
dkv ingest <project>
```

twice without source changes must not continuously increase record counts.

### Transactions

Database and filesystem behavior must satisfy the contract's rollback requirements.

### Storage statistics

Report actual stored content, not merely checksum metadata.

### Verification

`dkv vault verify` must detect the corruption cases specified by the contract.

---

# Step 7 — Tests Are Part of the Implementation

Do not merely make the existing tests pass.

For every failed contract criterion, add a regression test.

Especially test:

```text
unversioned package deduplication
repeated ingestion
changed document content
duplicate physical content
missing content
checksum mismatch
orphaned records
transaction rollback
staged-file cleanup
storage size
vault reopen
migration
```

Prefer deterministic temporary test vaults.

Tests must not depend on the user's real DKV vault.

---

# Step 8 — End-to-End Verification

At minimum verify:

```text
fixture project
      ↓
scanner
      ↓
project manifest
      ↓
provider registry
      ↓
CollectedDocs
      ↓
storage
      ↓
SQLite
      ↓
vault content
      ↓
vault verify
```

The test must exercise the real integration path rather than reproducing the behavior with mocked internals.

---

# Step 9 — CLI Verification

Verify:

```bash
dkv ingest <fixture-project>
dkv vault status
dkv vault verify
```

Check both:

* first ingestion
* repeated ingestion

Expected behavior:

```text
first run    → records/content created
second run   → no duplicate logical records/content
changed file → existing document updated
```

---

# Step 10 — Quality Gates

After fixes:

```bash
cargo fmt --check
cargo check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test
cargo doc --no-deps
```

All five must pass.

Do not suppress Clippy warnings merely to obtain a green gate unless the suppression is genuinely justified by the architecture and documented.

---

# Step 11 — Final Contract Audit

Before declaring completion, reread:

```bash
cat specs/M3-storage.md
```

Walk through **every acceptance criterion**.

There must be no unchecked required item.

If something cannot reasonably be implemented in this milestone:

1. do not silently omit it;
2. document the deviation;
3. explain why;
4. do not claim M3 complete.

---

# Step 12 — Commit

Only after all gates pass:

```bash
git status
git diff
git diff --stat
```

Ensure unrelated files are not included.

Especially do not commit unrelated user modifications.

Commit with:

```bash
git add <only-m3-files>
git commit -m "Reconcile storage layer with M3 contract"
```

Do not amend unrelated commits.

Push the current branch only if the repository workflow permits it.

---

# Final Report

Return a concise report containing:

## Baseline

What passed/failed before reconciliation.

## Contract Matrix

```text
PASS  ...
PASS  ...
FIXED ...
PASS  ...
```

## Changes

List every implementation change.

## Tests

List new/updated tests.

## Final Gates

```text
cargo fmt --check                                      PASS
cargo check                                            PASS
cargo clippy ... -D warnings                          PASS
cargo test                                             PASS
cargo doc --no-deps                                    PASS
```

## Git

Report:

```text
branch:
commit:
working tree:
```

## M3 Status

Use exactly one:

```text
M3 COMPLETE
```

or:

```text
M3 NOT COMPLETE
```

If `M3 NOT COMPLETE`, list the exact remaining contract items.

---

## Hard Stop

When M3 is reconciled and all five gates pass:

**STOP.**

Do not begin M4.

Do not implement:

* indexing
* search
* Tantivy
* SQLite FTS
* network providers
* docs.rs
* npm registry
* GitHub
* Zeal
* AI
* embeddings
* RAG

Those belong to subsequent milestones.
