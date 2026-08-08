# DKV — M3 Storage Contract

**Status:** Canonical
**Phase:** M3 — Storage & Vault Foundation
**Depends on:** M1 Project Understanding, M2 Knowledge Acquisition Foundation
**Blocks:** M4 Indexing & Search
**Scope:** Offline-first persistent knowledge storage

---

## 1. Purpose

M3 establishes the persistent storage layer for DKV.

The storage layer consumes provider output:

```text
Project
  ↓
Scanner
  ↓
ProjectManifest
  ↓
Provider Registry
  ↓
CollectedDocs
  ↓
Storage
  ↓
Persistent Vault
```

Storage is responsible for persistence, deduplication, checksums, metadata, transactions, and vault integrity.

Storage must not contain provider-specific acquisition logic.

---

# 2. Scope

M3 includes:

* vault filesystem layout
* SQLite database
* database migrations
* storage domain model
* document ingestion
* metadata persistence
* content storage
* checksums
* deduplication
* idempotent ingestion
* transactional ingestion
* vault verification
* vault statistics
* storage-related CLI commands
* storage tests

---

# 3. Explicit Non-Goals

The following are **NOT part of M3**:

* search
* SQLite FTS
* Tantivy
* ranking
* semantic search
* embeddings
* vectors
* RAG
* LLM integration
* AI
* docs.rs network acquisition
* npm registry acquisition
* GitHub API acquisition
* Zeal network acquisition

Those belong to later phases.

---

# 4. Storage Architecture

The storage layer must be isolated:

```text
src/
├── providers/
│
├── scanner/
│
├── project/
│
└── storage/
    ├── mod.rs
    ├── vault.rs
    ├── ingest.rs
    ├── metadata.rs
    ├── checksum.rs
    ├── layout.rs
    ├── sqlite.rs
    ├── transaction.rs
    └── errors.rs
```

Exact file decomposition may vary if existing architecture requires it, but responsibilities must remain separated.

---

# 5. Provider/Storage Boundary

Providers produce:

```text
CollectedDocs
```

Storage consumes:

```text
CollectedDocs
```

Storage must not know whether a document came from:

* Cargo
* npm
* GitHub
* Zeal
* man
* local filesystem

Provider-specific acquisition remains inside providers.

---

# 6. Vault Layout

A vault must have a deterministic filesystem layout:

```text
vault/
├── metadata/
├── documents/
├── packages/
├── artifacts/
├── checksums/
├── sqlite/
├── logs/
└── tmp/
```

The layout implementation must:

* create missing directories
* avoid destructive initialization
* use deterministic paths
* keep temporary files separate from permanent content

---

# 7. SQLite Database

SQLite is the initial storage backend.

Database location:

```text
vault/sqlite/
```

The database must use migrations.

Do not rely on ad-hoc `CREATE TABLE IF NOT EXISTS` logic as the migration mechanism.

Schema evolution must be explicit and versioned.

---

# 8. Required Tables

The storage schema must provide at least:

```text
packages
documents
sources
artifacts
checksums
tags
document_tags
ingest_runs
```

Additional tables are permitted when required by implementation.

---

# 9. Package Identity

A package must be uniquely identifiable without incorrectly treating SQL `NULL` values as distinct identities.

Logical identity:

```text
kind
name
version
root
```

The implementation must define deterministic semantics for packages without a version.

The schema must not permit duplicate logical package identities merely because:

```text
version IS NULL
```

and:

```text
version IS NULL
```

are represented as separate SQL rows.

---

# 10. Document Identity

A document must be idempotently identifiable by its logical location within a package/source.

Required identity semantics:

```text
package_id
source_id
relative_path
```

Repeated ingestion of the same document must update/reuse the existing record rather than create duplicate rows.

---

# 11. Knowledge Document Metadata

Persist at minimum:

```text
id
package_id
source_id
relative_path
title
document_type
mime_type
language
size
checksum
fs_path
vault_path
created_at
updated_at
content_stored
```

Exact column names may differ only when equivalent semantics are preserved.

---

# 12. Package Metadata

Persist:

```text
kind
name
version
root
```

Examples:

```text
cargo / serde / 1.x
npm / svelte / 5.x
local / project-name / null
```

The model must support packages without versions.

---

# 13. Source Metadata

A source identifies where knowledge originated.

Examples:

```text
cargo
local
github
npm
zeal
man
```

A source must have a stable identity.

The storage system must not duplicate equivalent source records unnecessarily.

---

# 14. Content Storage

Documents may have metadata without stored content.

`content_stored` must explicitly distinguish:

```text
metadata known
```

from:

```text
content physically stored
```

Content stored in the vault must use deterministic content-addressed storage.

Preferred structure:

```text
documents/<content-checksum>
```

or an equivalent deterministic content-addressed layout.

---

# 15. Checksums

Checksum handling must be centralized and reusable.

At minimum support:

```text
content checksum
metadata checksum
```

Content checksums must be deterministic.

The checksum must be calculated from the actual document content, not its path.

The checksum implementation must be reusable by:

* ingestion
* verification
* future providers
* future indexing

---

# 16. Deduplication

The vault must avoid duplicate physical content.

If two documents contain identical content:

```text
A → checksum X
B → checksum X
```

the vault should store one physical content object where practical.

Database records may still represent two logical documents.

---

# 17. Ingestion Pipeline

The canonical ingestion flow is:

```text
CollectedDocs
      ↓
normalize
      ↓
deduplicate
      ↓
checksum
      ↓
persist
      ↓
record ingest
```

Each stage must have a clear responsibility.

---

# 18. Normalization

Before persistence:

* normalize paths
* normalize metadata
* normalize document types
* normalize source identity
* normalize package identity
* calculate deterministic metadata

Normalization must not destroy meaningful source information.

---

# 19. Idempotency

Ingestion must be idempotent.

Running:

```bash
dkv ingest <project>
```

multiple times against an unchanged project must not continuously increase:

* package count
* source count
* document count
* physical content count

unless actual knowledge changed.

---

# 20. Changed Documents

When content changes:

```text
old checksum != new checksum
```

the storage system must correctly update the logical document record and content reference.

Old content may remain available when required for deduplication/history, but the active document must reference the current content.

---

# 21. Transactions

Ingestion must be transactional at the database level.

A failed ingest must not leave partially committed database state.

The intended flow is:

```text
begin transaction
      ↓
prepare metadata
      ↓
stage content
      ↓
persist records
      ↓
record ingest
      ↓
commit
```

On failure:

```text
rollback DB
cleanup newly-created staged/permanent content
```

---

# 22. Atomic File Handling

Content writes must not expose partially written documents as valid vault content.

Use temporary staging:

```text
vault/tmp/<id>
```

then atomically move/rename into the permanent content location.

If database commit fails, newly-created content must be cleaned up where safe.

Existing shared content must never be deleted merely because one transaction failed.

---

# 23. Ingest Runs

Every ingestion operation must be observable.

`ingest_runs` must record sufficient information to determine:

* when ingestion started
* when it finished
* success/failure
* project/root
* provider/source where applicable
* number of documents processed
* number added
* number updated
* number skipped
* errors

Exact fields may vary while preserving these semantics.

---

# 24. Vault API

The vault abstraction should expose operations equivalent to:

```rust
open()
ingest()
status()
verify()
```

The CLI must not directly manipulate SQLite.

CLI → Vault → Storage backend.

---

# 25. CLI

M3 must implement:

```bash
dkv ingest <project>
```

```bash
dkv vault status
```

```bash
dkv vault verify
```

### `dkv ingest`

Must:

1. inspect the project
2. obtain the project manifest
3. invoke applicable providers
4. collect documents
5. persist knowledge
6. report ingestion results

### `dkv vault status`

Must report at minimum:

```text
packages
documents
artifacts
stored content
storage size
```

### `dkv vault verify`

Must detect:

* missing content files
* checksum mismatches
* orphaned database rows
* invalid content references
* other vault consistency errors

---

# 26. Storage Size

Storage statistics must represent actual stored vault content.

Do not calculate storage size merely from every checksum record.

Only content physically stored in the vault should contribute to stored-content size.

Conceptually:

```sql
SUM(checksums.size)
WHERE documents.content_stored = 1
```

or equivalent semantics.

---

# 27. Storage Backend Boundary

The ingestion pipeline should depend on a storage abstraction where practical.

SQLite is the first implementation.

Conceptually:

```text
StorageBackend
      ↑
   SQLite
```

This prevents provider and ingestion logic from becoming directly coupled to SQLite.

---

# 28. Error Handling

Storage errors must be represented through DKV's existing error system.

Errors must preserve useful context for:

* filesystem failures
* SQLite failures
* migration failures
* checksum failures
* transaction failures
* invalid vault state

Do not silently discard storage errors.

---

# 29. Concurrency

The implementation must avoid unsafe concurrent writes.

SQLite transactions must use appropriate connection/transaction semantics.

Concurrent read support may be used where naturally supported, but M3 does not require a sophisticated concurrent ingestion engine.

---

# 30. Tests

M3 must include tests covering:

### Vault

* vault creation
* directory creation
* opening existing vault
* reopening vault

### Database

* initial migration
* migration idempotency
* schema initialization

### Packages

* package insertion
* package deduplication
* versioned packages
* unversioned packages
* NULL-version identity

### Sources

* source insertion
* source deduplication

### Documents

* document insertion
* document update
* repeated ingestion
* logical identity
* changed content

### Content

* content storage
* content deduplication
* checksum calculation
* checksum verification
* missing content detection

### Transactions

* successful transaction
* database rollback
* staged-file cleanup
* failure does not corrupt existing content

### CLI

* `ingest`
* `vault status`
* `vault verify`

### Integration

At least one end-to-end test:

```text
fixture project
    ↓
scanner
    ↓
providers
    ↓
CollectedDocs
    ↓
vault
    ↓
SQLite
    ↓
stored content
    ↓
verify
```

---

# 31. Quality Gates

M3 is incomplete unless all gates pass:

```bash
cargo fmt --check

cargo check

cargo clippy --workspace --all-targets --all-features -- -D warnings

cargo test

cargo doc --no-deps
```

No warnings may remain under the Clippy command.

---

# 32. Existing Functionality Must Remain Green

M3 must not regress:

* scanner
* ignore handling
* project detection
* Cargo parsing
* Node parsing
* provider registry
* Cargo provider
* Local provider
* existing CLI commands
* existing tests

All existing tests must continue passing.

---

# 33. Acceptance Criteria

M3 is **DONE** only when all of the following are true:

* [ ] Vault layout exists and is deterministic
* [ ] SQLite backend exists
* [ ] Database migrations exist
* [ ] Required schema exists
* [ ] Package identity is deterministic
* [ ] NULL-version packages deduplicate correctly
* [ ] Source identity is deterministic
* [ ] Document identity is deterministic
* [ ] Document ingestion is idempotent
* [ ] Content is checksum-addressed
* [ ] Duplicate physical content is avoided
* [ ] Metadata is persisted
* [ ] Filesystem paths are persisted correctly
* [ ] Ingestion is transactional
* [ ] Failed transactions do not corrupt the vault
* [ ] Staged content is cleaned up on failure
* [ ] Ingest runs are recorded
* [ ] `dkv ingest` works
* [ ] `dkv vault status` works
* [ ] `dkv vault verify` works
* [ ] Storage statistics are correct
* [ ] Missing content is detected
* [ ] Checksum mismatches are detected
* [ ] Orphaned records are detected
* [ ] Unit tests cover storage behavior
* [ ] Integration tests cover end-to-end ingestion
* [ ] `cargo fmt --check` passes
* [ ] `cargo check` passes
* [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
* [ ] `cargo test` passes
* [ ] `cargo doc --no-deps` passes

---

# 34. Phase Boundary

Once M3 passes all acceptance criteria, freeze the storage API.

Do not add:

* search
* indexing
* network providers
* AI
* RAG

during M3 reconciliation.

The next milestone is:

```text
M4 — Indexing & Search
```

M4 consumes the persisted knowledge produced by M3.

---

# 35. Implementation Rule

When reconciling an existing implementation against this contract:

1. Preserve working architecture.
2. Do not rewrite completed scanner/provider systems.
3. Do not introduce unnecessary frameworks.
4. Prefer the smallest implementation satisfying the contract.
5. Add tests for every corrected behavior.
6. Run all five quality gates.
7. Report any deliberate deviation explicitly.
8. Do not declare M3 complete while any acceptance criterion remains unchecked.
