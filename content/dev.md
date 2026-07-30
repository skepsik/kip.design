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
    vault-cli/            # clap; бинарь `kip` (`kip init`, …)
  tests/
    integration/          # happy-path, два устройства (опционально e2e)
```

**Правило зависимостей:** `vault-domain` не зависит ни от чего, кроме `vault-ports` и
stdlib/serde (только для сериализации меты в типах — либо вынести `Meta` в `vault-ports`).

```
vault-cli → vault-runtime → vault-domain, vault-store, vault-git, vault-crypto
vault-runtime → vault-ports
vault-git, vault-crypto, vault-store → vault-ports
```

---

## 3. Порты (`vault-ports`)

Имена и сигнатуры — контракт; реализация в адаптерах. **Типы id / меты / таблицы —
как в `domain.rs`** (не `Uuid`-newtype из старых черновиков). Полные порты
`HistoryPort` / `PackerPort` / `WorkTreePort` / `TablePersistence` — там же; §3.2–3.5 —
упрощённый контур имён для MVP (`BlobStore`, `Crypto`, `GitHistory`, `VaultState`),
без расхождения по форме идентификаторов.

### 3.1. Идентификаторы

Канон — [`domain.rs`](https://github.com/skepsik/kip.design/blob/master/domain.rs): opaque-имя в россыпи; сорт из имени **не** выводится.

```rust
// Случайные, не инкремент. UUID/CSPRNG — только генерация содержимого строки.
pub struct ObjectName(String);       // имя файла в корне repo (мета, тело, слот, …)
pub struct NodeId(ObjectName);       // идентичность узла = имя его меты
pub struct BodyName(ObjectName);     // имя объекта-тела
pub struct SyncMark(String);         // метка синка (в git — хэш коммита)
pub enum ParentRef { Root, Node(NodeId) }  // корень — сентинел, не объект
```

### 3.2. `BlobStore`

Чтение/запись **шифротекста** в корне git-репозитория. Без расшифровки.

```rust
pub trait BlobStore {
    fn read(&self, name: &ObjectName) -> Result<CipherBytes, StoreError>;
    fn write_atomic(&self, name: &ObjectName, ciphertext: &CipherBytes) -> Result<(), StoreError>;
    fn remove(&self, name: &ObjectName) -> Result<(), StoreError>;
    fn list_tracked_names(&self) -> Result<Vec<ObjectName>, StoreError>; // fsck / cold id set
}
```

`CipherBytes` — из domain. Имена — opaque string (например hex без дефисов). Папки `objects/` нет (ux §2).

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
// Соответствует RawStatus / RawDelta в domain.rs (Appeared/Modified/Disappeared).
pub enum DeltaStatus { Added, Modified, Deleted }

pub struct BlobDelta {
    pub name: ObjectName,   // имя файла в корне repo
    pub status: DeltaStatus,
}

pub trait GitHistory {
    fn diff_since(&self, mark: &SyncMark) -> Result<Vec<BlobDelta>, GitError>;
    fn commit_one(&self, name: &ObjectName, message: &str) -> Result<SyncMark, GitError>;
    fn read_blob_at(&self, rev: &str, name: &ObjectName) -> Result<CipherBytes, GitError>;
    fn head(&self) -> Result<SyncMark, GitError>;
}
```

Полный порт истории (`HistoryPort`: pull, purge_from_history, publish, MarkVanished) — в `domain.rs`; этот скетч — минимальный контур для MVP-адаптера.

`.gitattributes`: `* binary -diff -merge -text`, `--no-renames` на diff (implementation §2.1).

### 3.5. `VaultState` (локальное, `.vault/`)

```rust
pub trait VaultState {
    fn sync_marker(&self) -> Result<Option<SyncMark>, StateError>;
    fn set_sync_marker(&self, mark: &SyncMark) -> Result<(), StateError>;
    fn load_table(&self) -> Result<Table, StateError>;
    fn save_table(&self, table: &Table) -> Result<(), StateError>;
    fn active_ram_root(&self) -> Result<Option<PathBuf>, StateError>;
    fn set_active_ram_root(&self, path: Option<&Path>) -> Result<(), StateError>;
}
```

`Table` — тип из `vault-domain` (канон `domain.rs`). Межсессионная персистентность таблицы на MVP нет (implementation §1.1); сигнатуры — на будущее / in-memory.

---

## 4. Домен (`vault-domain`) — ключевые типы

Сверка с замороженным `domain.rs`. Ниже — сжатый скетч; при сомнении — файл.

### 4.1. Мета (schema v1)

Канон — `domain.rs` (`Meta`, `Place`, `NodeBody`, `SchemaVersion`).

```rust
pub struct SchemaVersion(pub u16);
pub const CURRENT_SCHEMA: SchemaVersion = SchemaVersion(1);

pub struct Place {
    pub parent: ParentRef,
    pub name: NodeName,       // санитизация на parse (path traversal)
}

pub enum NodeBody {
    Secret(BodyName),
    Directory,
}

pub struct Meta {
    pub schema: SchemaVersion,
    pub place: Place,
    pub body: NodeBody,
}
```

Сериализация на диске: **JSON** (читаемость при отладке, `serde_json`). Миграция
ленивая (implementation §1.4). Поле `schema` пишем текущее, читаем все ≤ текущей.

### 4.2. Таблица

Канон — `domain.rs` (`Table`, `Sort`, `NodeEntry`, `PlainHash`). На MVP таблица
живёт только в RAM вместе с деревом (implementation §1.1); отдельного
`table.enc` нет.

```rust
pub enum Sort {
    Meta(NodeId),
    Body(BodyName),
    Chaff, // критерий распознавания — открытый вопрос до fsck (design §5.4)
}

pub struct NodeEntry {
    pub meta: Meta,
    pub plain_hash: Option<PlainHash>, // None — тело ещё не разворачивалось
}

pub struct Table {
    pub sorts: BTreeMap<ObjectName, Sort>,
    pub nodes: BTreeMap<NodeId, NodeEntry>,
    pub children: BTreeMap<ParentRef, Vec<NodeId>>, // производный индекс
    pub mark: Option<SyncMark>,
}
```

### 4.3. Операции домена (без I/O)

```rust
pub fn apply_remote_delta(table: &mut Table, delta: &RawDelta, ...) -> Result<AppliedGraph, DomainError>;
pub fn validate_graph(applied: AppliedGraph) -> Result<ValidatedState, ...>; // design §4; типы-состояния — domain.rs
pub fn resolve_merge_anomalies(table: &mut Table, history: &dyn HistoryBytes) -> Result<..., ...>;
pub fn plan_pack(scan: &RamTreeScan, table: &Table) -> PackPlan;
pub fn materialize_paths(state: &ValidatedState) -> HashMap<PathBuf, MaterializeOp>;
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

**Намеренно без** `git2`, `sequoia` на MVP — только subprocess (дефолтный адаптер упаковщика +
git как есть). Меньше FFI-поверхности для непроверяемого кода.

---

## 7. Фазы (backlog)

Чеклист; трекер не обязателен.

- [ ] **P1** Workspace, `vault-ports`, типы/`Meta` по `domain.rs`, пустой `vault-domain`
- [ ] **P2** `Table`, apply delta (2 pass), `validate_graph`, merge rules — **in-memory fake store**
- [ ] **P2** Property-тест сходимости (главный критерий готовности домена)
- [ ] **P2** Счётчик распаковок в fake crypto — регресс move-dir O(1)
- [ ] **P3** `vault-store` + `vault-git` на tempfile-репозитории; повтор property-теста
- [ ] **P4** `vault-crypto` (gpg); интеграция seal/unseal
- [ ] **P5** `vault-cli`: init, unfold, pack, fold, push, fsck
- [ ] **P6** history / restore, sync, конфликтные копии в RAM

---

## 8. Формальные контракты для ревью

При приёме работы (человеком или агентом) смотреть не стиль, а:

1. **`vault-domain` не импортирует** `std::process`, `std::fs` (кроме тестов).
2. **Property-тест P2 зелёный** после любого изменения домена.
3. **Нет ручного `git commit`** вне `GitHistory::commit_one`.
4. **Восстановительные операции** — байты из истории, не re-seal (implementation §1.2).
5. **UX:** pack/fold/sync поведение совпадает с [ux](./ux.md).

---

## Приложение. Ссылки

- Домен: [design](./design.md)
- Решения до кода: [implementation](./implementation.md)
- Пользователь: [ux](./ux.md)
