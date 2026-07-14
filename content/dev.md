# Хранилище секретов — скелет реализации (Rust)

Документ для перехода к коду. Домен — **[design](./design.md)**; адаптеры и
фазы — **[implementation](./implementation.md)**; CLI и диск — **[ux](./ux.md)**.

При появлении репозитория этот файл переносится или сливается в корневой
**`ARCHITECTURE.md`**; параллельно не ведётся.

---

## 1. Почему Rust при отсутствии контроля на уровне кода

Разработчик опирается на **формальные контракты** (типы, trait-границы) и **машинные
проверки** (компилятор, тесты), а не на построчный обзор каждого изменения.

| Механизм                      | Что гарантирует                                                                 |
| ----------------------------- | ------------------------------------------------------------------------------- |
| **Типы домена**               | Нельзя перепутать meta/body, revision, NodeId; инварианты графа — в одном crate |
| **`vault-domain` без I/O**    | Домен не зовёт git/gpg/fs — граница из design §5.1 соблюдается физически        |
| **Trait-порты**               | Адаптеры подменяемы; контракт виден в сигнатурах                                |
| **Property-тесты (proptest)** | Сходимость merge и стоимость O(1) — главная страховка (implementation §3)       |
| **Интеграционные тесты**      | Временный git-репо + вызовы `gpg` в CI/локально                                 |

TypeScript не даёт статических гарантий на граф и гонки; Python — слабее на контрактах
и дистрибуции. Rust здесь — не «знаю язык», а **инструмент доверия к чужому/сгенерированному
коду**.

---

## 2. Workspace (crate layout)

```
secrets-vault/
  Cargo.toml              # workspace
  ARCHITECTURE.md         # слив с этого файла
  crates/
    vault-domain/         # чистый домен: граф, дельта, merge, apply (2 прохода)
    vault-ports/          # trait-определения (зависимость domain ↔ adapters)
    vault-store/          # пути в корне repo, .vault/, атомарная запись
    vault-git/            # git subprocess: delta, commit-one, cat-file
    vault-crypto/           # gpg subprocess: seal/unseal meta и body
    vault-runtime/        # склейка: Table, unfold/fold/pack orchestration
    vault-cli/            # clap: init, unfold, pack, fold, sync, …
  tests/
    integration/          # happy-path, два устройства (опционально e2e)
```

**Правило зависимостей:** `vault-domain` не зависит ни от чего, кроме `vault-ports` и
stdlib/serde (только для сериализации меты в типах — либо вынести `MetaV1` в `vault-ports`).

```
vault-cli → vault-runtime → vault-domain, vault-store, vault-git, vault-crypto
vault-runtime → vault-ports
vault-git, vault-crypto, vault-store → vault-ports
```

---

## 3. Порты (`vault-ports`)

Имена и сигнатуры — контракт; реализация в адаптерах.

### 3.1. Идентификаторы

```rust
// Случайные, не инкремент. Display/FromStr для путей в корне repo.
pub struct NodeId(pub Uuid);      // мета: имя файла = NodeId
pub struct BlobId(pub Uuid);      // тело: отдельный случайный id
pub struct SentinelRoot;          // корень дерева (без объекта)
```

### 3.2. `BlobStore`

Чтение/запись **шифротекста** в корне git-репозитория. Без расшифровки.

```rust
pub trait BlobStore {
    fn read(&self, name: &str) -> Result<Vec<u8>, StoreError>;
    fn write_atomic(&self, name: &str, ciphertext: &[u8]) -> Result<(), StoreError>;
    fn remove(&self, name: &str) -> Result<(), StoreError>;
    fn list_tracked_names(&self) -> Result<Vec<String>, StoreError>; // для fsck / cold id set
}
```

Имена — opaque string (hex/uuid без дефисов). Папки `objects/` нет (ux §2).

### 3.3. `Crypto`

```rust
pub trait Crypto {
    fn seal_meta(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn unseal_meta(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn seal_body(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn unseal_body(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError>;
}
```

Реализация: `gpg --batch` (см. implementation §2.2). На диске и в git — полный блоб
`заголовок || ciphertext` (`vk_epoch` в заголовке). Passphrase — env / file / agent
(ux §4).

### 3.4. `GitHistory`

```rust
pub enum DeltaStatus { Added, Modified, Deleted }

pub struct BlobDelta {
    pub path: String,       // имя файла в корне repo
    pub status: DeltaStatus,
}

pub trait GitHistory {
    fn diff_since(&self, commit: &str) -> Result<Vec<BlobDelta>, GitError>;
    fn commit_one(&self, path: &str, message: &str) -> Result<String, GitError>; // new HEAD
    fn read_blob_at(&self, commit: &str, path: &str) -> Result<Vec<u8>, GitError>;
    fn head(&self) -> Result<String, GitError>;
}
```

`.gitattributes`: `* binary -diff -merge -text`, `--no-renames` на diff (implementation §2.1).

### 3.5. `VaultState` (локальное, `.vault/`)

```rust
pub trait VaultState {
    fn sync_marker(&self) -> Result<Option<String>, StateError>;
    fn set_sync_marker(&self, commit: &str) -> Result<(), StateError>;
    fn load_table(&self) -> Result<GraphTable, StateError>;
    fn save_table(&self, table: &GraphTable) -> Result<(), StateError>;
    fn active_ram_root(&self) -> Result<Option<PathBuf>, StateError>;
    fn set_active_ram_root(&self, path: Option<&Path>) -> Result<(), StateError>;
}
```

`GraphTable` — тип из `vault-domain` (или общий в `vault-ports`).

---

## 4. Домен (`vault-domain`) — ключевые типы

### 4.1. Мета (schema v1)

```rust
pub const META_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MetaV1 {
    pub schema_version: u32,
    pub name: String,
    pub parent: ParentRef,
    pub body: Option<BlobId>,   // None = каталог
}

pub enum ParentRef {
    Root,
    Node(NodeId),
}
```

Сериализация: **JSON** (читаемость при отладке, `serde_json`). Миграция ленивая
(implementation §1.4).

### 4.2. Таблица (горячая + `table.enc`)

```rust
pub struct GraphTable {
    pub object_kind: HashMap<String, ObjectKind>, // имя файла в repo → Meta | Body
    pub nodes: HashMap<NodeId, NodeRow>,
    // обратный индекс children_by_parent — производный, в памяти
}

pub struct NodeRow {
    pub meta_path: String,           // имя файла меты (= NodeId)
    pub parent: ParentRef,
    pub name: String,
    pub body: Option<BlobId>,
    pub plaintext_hash: Option<[u8; 32]>, // для pack без распаковки; None у каталога
}
```

### 4.3. Операции домена (без I/O)

```rust
pub fn apply_remote_delta(table: &mut GraphTable, delta: &[BlobDelta], ...) -> Result<ApplyReport, DomainError>;
pub fn validate_graph(table: &GraphTable) -> Result<(), MergeValidationError>; // design §4
pub fn resolve_merge_anomalies(table: &mut GraphTable, history: &dyn HistoryBytes) -> Result<..., ...>;
pub fn plan_pack(scan: &RamTreeScan, table: &GraphTable) -> PackPlan;  // что писать
pub fn materialize_paths(table: &GraphTable) -> HashMap<PathBuf, MaterializeOp>; // unfold
```

`HistoryBytes` — trait для «достать старые байты без re-encrypt» (implementation §1.2).

### 4.4. Property-тест (обязателен до git/gpg)

```rust
// proptest: N шагов на 2–3 ModelDevice
// ops: create file, edit, mkdir, mv, rm, sync at random points
// assert: после полного обмена — одинаковые множества тел (и путей) на всех устройствах;
//         validate_graph() ok; unpack count на move-dir с N детьми = O(1)
```

---

## 5. Runtime (`vault-runtime`) — оркестрация команд

| UX-команда | Runtime                                                                                   |
| ---------- | ----------------------------------------------------------------------------------------- |
| `unfold`   | pull → `apply_remote_delta` (2 прохода) → decrypt → materialize tmpfs → load table в RAM  |
| `pack`     | scan RAM → `plan_pack` → seal → `BlobStore` + `GitHistory::commit_one` × N → update table |
| `fold`     | `pack` + clear `active_ram_root`, teardown tmpfs                                          |
| `sync`     | pull + apply + `pack` + optional push                                                     |

Упаковка + коммиты **неделимы** внутри `pack` (ux §5.1).

---

## 6. Зависимости (ориентир)

| Crate         | Зависимости                                        |
| ------------- | -------------------------------------------------- |
| vault-domain  | serde, serde_json, uuid, thiserror, proptest (dev) |
| vault-ports   | uuid, thiserror                                    |
| vault-store   | vault-ports, tempfile                              |
| vault-git     | vault-ports, std::process                          |
| vault-crypto  | vault-ports, std::process                          |
| vault-runtime | все выше                                           |
| vault-cli     | clap, vault-runtime                                |

**Намеренно без** `git2`, `sequoia` на MVP — только subprocess, как в дизайне «gpg +
git как есть». Меньше FFI-поверхности для непроверяемого кода.

---

## 7. Фазы (backlog)

Чеклист; трекер не обязателен.

- [ ] **P0** Workspace, `vault-ports`, `MetaV1`, пустой `vault-domain`
- [ ] **P1** `GraphTable`, apply delta (2 pass), `validate_graph`, merge rules — **in-memory fake store**
- [ ] **P1** Property-тест сходимости (главный критерий готовности домена)
- [ ] **P1** Счётчик распаковок в fake crypto — регресс move-dir O(1)
- [ ] **P2** `vault-store` + `vault-git` на tempfile-репозитории; повтор property-теста
- [ ] **P3** `vault-crypto` (gpg); интеграция seal/unseal
- [ ] **P4** `vault-cli`: init, unfold, pack, fold, push, fsck
- [ ] **P5** history / restore, sync, конфликтные копии в RAM

---

## 8. Формальные контракты для ревью

При приёме работы (человеком или агентом) смотреть не стиль, а:

1. **`vault-domain` не импортирует** `std::process`, `std::fs` (кроме тестов).
2. **Property-тест P1 зелёный** после любого изменения домена.
3. **Нет ручного `git commit`** вне `GitHistory::commit_one`.
4. **Восстановительные операции** — байты из истории, не re-seal (implementation §1.2).
5. **UX:** pack/fold/sync поведение совпадает с [ux](./ux.md).

---

## Приложение. Ссылки

- Домен: [design](./design.md)
- Решения до кода: [implementation](./implementation.md)
- Пользователь: [ux](./ux.md)
