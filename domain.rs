//! Доменные типы хранилища секретов.
//!
//! Статус файла: ЗАМОРОЖЕН. Ревьюится как дизайн-документ; агенты реализации
//! не меняют ничего в этом файле без явного решения владельца. Реализации
//! живут в других модулях и подчиняются этим типам, не наоборот.
//!
//! Ссылки вида (Д 2.1) — на разделы сводного дизайна,
//! (Р 1.2) — на разделы реализационного документа.
//!
//! Конвенции файла:
//! - идентификаторы — newtype'ы: перепутать имя меты с именем тела нельзя;
//! - все классификации — исчерпывающие enum'ы: новый кейс не компилируется,
//!   пока не обработан везде;
//! - двухфазность разворота (Д 3.5) выражена типами-состояниями: функция
//!   материализации не принимает непровалидированный граф.

#![allow(dead_code)]

use std::collections::BTreeMap;

// ============================================================================
// 1. Идентичности и имена объектов (Д 2.1, 2.2, 7)
// ============================================================================

/// Непрозрачное имя объекта в россыпи хранилища — то, что видит транспорт.
/// Сорт объекта (мета/тело) из имени НЕ выводится (Д 2.2): единственный
/// легальный способ узнать сорт — таблица. Конструируется только адаптерами.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectName(String);

/// Идентичность узла = имя его меты (`kNNN`). Рождается один раз, навсегда
/// (Д 2.1). Получается ТОЛЬКО из таблицы (резолюция сорта) либо при рождении
/// узла — прямого конструктора из ObjectName у реализации нет.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(ObjectName);

/// Имя объекта-тела. Случайное, рождается вместе с узлом, неизменно (Д 2.2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyName(ObjectName);

/// Метка последней синхронизации (Д 6; в git-адаптере — хэш коммита, Р 2.1).
/// Сдвигается только после успешной валидации и материализации (Р 2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncMark(String);

/// Адрес прошлого состояния в истории (Р 2.1: ревизия для воскрешений/diff).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision(String);

// ============================================================================
// 2. Место узла (Д 2.1): пара (родитель, имя). Полного пути в домене НЕТ.
// ============================================================================

/// Ссылка на родителя. Корень — сентинел, не объект (Д 2.1): выражен
/// вариантом enum'а, спутать с реальным узлом невозможно.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParentRef {
    Root,
    Node(NodeId),
}

/// Видимое имя узла внутри родителя.
///
/// ВНИМАНИЕ (найдено при переводе в типы): имя участвует в материализации
/// рабочего дерева, значит обязано быть санитизировано — пустая строка,
/// `/`, `\`, `..`, `.` и управляющие символы недопустимы, иначе мета,
/// пришедшая с другого устройства, устраивает path traversal при развороте.
/// Единственный конструктор — валидирующий; несанитизированное имя в мете —
/// ошибка данных класса DataError.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeName(String);

impl NodeName {
    pub fn parse(raw: &str) -> Result<Self, DataError> {
        // реализация: правила выше; здесь только контракт
        let _ = raw;
        unimplemented!()
    }
}

/// Место узла — мутабельный атрибут (Д 2.1). Единица сравнения для
/// «переезд/переименование» и для дублей места (Д 4).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub parent: ParentRef,
    pub name: NodeName,
}

/// Полный путь существует только как производная проекция (Д 2.1, 2.4).
/// Тип конструируется исключительно функцией проекции из валидированного
/// состояния — см. `ValidatedState::derive_path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedPath(Vec<NodeName>);

// ============================================================================
// 3. Мета и тело (Д 2.2)
// ============================================================================

/// Версия схемы меты (Р 1.4): читаем все ≤ текущей, пишем текущую.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaVersion(pub u16);

pub const CURRENT_SCHEMA: SchemaVersion = SchemaVersion(1);

/// Секрет — узел с телом; каталог — узел без тела (Д 2.1).
/// Не Option<BodyName>: различие содержательное, пусть будет именованным.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeBody {
    Secret(BodyName),
    Directory,
}

/// Расшифрованное содержимое меты (Д 2.2). Ссылка на тело записывается при
/// рождении и НЕ меняется (Д 2.2) — инвариант проверяется валидацией
/// (GraphViolation::BodyLinkChanged), а не предполагается: писавшая сторона
/// могла быть сбойной или чужой.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub schema: SchemaVersion,
    pub place: Place,
    pub body: NodeBody,
}

/// Плейнтекст тела. Байт-в-байт исходный файл, без обёрток (Д 2.2).
/// НАМЕРЕННО не реализует Debug/Display/Clone: секретный материал не должен
/// уметь попадать в логи и не должен молча размножаться по памяти. Любой
/// тип, содержащий Plaintext или Passphrase, тем самым тоже теряет Debug —
/// это контракт, а не недосмотр.
pub struct Plaintext(pub Vec<u8>);

/// Шифртекст любого объекта, как он лежит в россыпи.
pub struct CipherBytes(pub Vec<u8>);

/// Хэш плейнтекста (Р 1.3): SHA-256 → `[u8; 32]`; атрибут узла в таблице, НЕ ключ
/// (одинаковые пароли легальны). Считается вне домена (runtime / work tree);
/// детекция правок и переездов при свёртке.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlainHash([u8; 32]);

// ============================================================================
// 4. Таблица — производный кэш (Д 5.3, Р 2.3)
// ============================================================================

/// Сорт объекта россыпи. Резолвится только таблицей.
///
/// DESIGN GAP (найдено при переводе): представление chaff-пустышек (Д 5.4)
/// не решено до конца. Если пустышка — «тело без меты», её нельзя отличить
/// от настоящего осиротевшего тела при fsck; если «мета», она материализуется
/// как узел. Напрашивается третий легальный формат расшифрованного содержимого
/// («пустышка»), распознаваемый ПОСЛЕ расшифровки и только владельцем ключа —
/// но это надо решить до реализации fsck. Вариант enum'а зарезервирован.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sort {
    Meta(NodeId),
    Body(BodyName),
    Chaff, // см. gap выше: критерий распознавания — открытый вопрос
}

/// Строка таблицы для узла. Таблица — кэш, не правда (Д 5.3): всё в ней
/// восстановимо из мет + списка имён объектов.
#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub meta: Meta,
    /// Р 1.3; None — тело ещё не разворачивалось в этой таблице.
    pub plain_hash: Option<PlainHash>,
}

/// Граф смежности с обратным индексом (Д 4, Р 2.3). Обратный индекс —
/// производное от entries, инварианты согласованности держит реализация.
#[derive(Debug, Default)]
pub struct Table {
    pub sorts: BTreeMap<ObjectName, Sort>,
    pub nodes: BTreeMap<NodeId, NodeEntry>,
    pub children: BTreeMap<ParentRef, Vec<NodeId>>,
    pub mark: Option<SyncMark>,
}

// ============================================================================
// 5. Дельта (Д 2.5, 3.1–3.2)
// ============================================================================

/// Сырой статус от поставщика дельты. R не существует (Д 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawStatus {
    Appeared,
    Modified,
    Disappeared,
}

/// Сырая дельта: непрозрачные имена, никакой семантики.
#[derive(Debug, Clone)]
pub struct RawDelta(pub Vec<(ObjectName, RawStatus)>);

/// Классифицированное событие (Д 3.2): сорт назвал событие, распаковка мет —
/// по необходимости, тел — никогда на этом этапе.
#[derive(Debug, Clone)]
pub enum Event {
    /// A меты (+ A тела для секрета) — рождение узла.
    NodeBorn { id: NodeId, meta: Meta },
    /// M меты — переезд/переименование/no-op; тела не касаться (Д 3.3).
    MetaChanged { id: NodeId, new_meta: Meta },
    /// M тела — правка содержимого; мета не тронута.
    ContentChanged { body: BodyName },
    /// D меты (+ D тела) — смерть узла.
    NodeDied { id: NodeId },
    /// A тела — поставка зависимости для NodeBorn; сама по себе не событие.
    BodyArrived { body: BodyName },
    /// D тела — парная к NodeDied.
    BodyGone { body: BodyName },
}

/// «Прочие комбинации — ошибка данных, ловим» (Д 3.2, 3.5).
#[derive(Debug, Clone)]
pub enum DataError {
    UnknownObject(ObjectName),
    IllegalName { id: NodeId, raw: String },
    MalformedMeta(NodeId),
    /// M тела, у которого нет живой меты, и т.п.
    InconsistentDelta(ObjectName),
}

// ============================================================================
// 6. Двухфазный разворот (Д 3.5) — типы-состояния
// ============================================================================

/// Фаза 1: вся дельта наложена на граф, инварианты НЕ проверены, пути НЕ
/// пересчитаны. Из этого типа нельзя материализовать дерево — функции
/// материализации его не принимают. Единственный выход — validate().
#[derive(Debug)]
pub struct AppliedGraph {
    pub table: Table,
    pub events: Vec<Event>,
}

/// Фаза 2 завершена: инварианты проверены, найденные нарушения разрешены
/// детерминированно (Д 4). Только из этого состояния существуют пути.
#[derive(Debug)]
pub struct ValidatedState {
    pub table: Table,
    /// Что было нарушено и как разрешено — для предъявления пользователю.
    pub resolutions: Vec<Resolution>,
}

impl ValidatedState {
    /// Единственный источник DerivedPath в системе (Д 2.1).
    pub fn derive_path(&self, id: &NodeId) -> DerivedPath {
        let _ = id;
        unimplemented!()
    }
}

/// Контракт разворота. Применение дельты идемпотентно (Р 2.1): при
/// несдвинутой метке повторный вызов даёт то же состояние.
pub fn apply_delta(base: Table, delta: RawDelta) -> Result<AppliedGraph, DataError> {
    let _ = (base, delta);
    unimplemented!()
}

pub fn validate(graph: AppliedGraph) -> Result<ValidatedState, DataError> {
    let _ = graph;
    unimplemented!()
}

// ============================================================================
// 7. Целостность графа после merge (Д 4)
// ============================================================================

/// Нарушения, возможные ТОЛЬКО как результат слияния параллельных историй.
#[derive(Debug, Clone)]
pub enum GraphViolation {
    /// Ребёнок ссылается на несуществующего родителя.
    Orphan { child: NodeId, missing_parent: NodeId },
    /// Взаимное вложение; members — в детерминированном порядке.
    Cycle { members: Vec<NodeId> },
    /// Два узла на одном месте.
    PlaceCollision { place: Place, claimants: Vec<NodeId> },
    /// Параллельная правка одного объекта (предъявлена транспортом, Д 5.2).
    ConcurrentEdit { object: ObjectName },
    /// Ссылка мета→тело изменилась против прошлого известного состояния —
    /// нарушение инварианта рождения (Д 2.2). Найдено при переводе в типы:
    /// инвариант нужно проверять, а не постулировать.
    BodyLinkChanged { id: NodeId, was: BodyName, now: BodyName },
}

/// Детерминированные разрешения (Д 4, Р 1.2). ТРЕБОВАНИЕ ко всем вариантам:
/// байтовый детерминизм — одинаковый вход на разных устройствах обязан дать
/// байтово идентичный результат; воскрешения и откаты — старыми байтами из
/// истории, переупаковка запрещена (Р 1.2). Новый нонс легален только там,
/// где рождается новый NodeId (конфликтная копия).
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Сирота: ребёнок побеждает, родитель воскрешается из истории.
    ResurrectParent { parent: NodeId, from: Revision },
    /// Цикл: проигравший (детерминированный выбор) — в корень с пометкой.
    BreakCycle { loser: NodeId, relocated_to: Place },
    /// Дубль места: проигравшему — детерминированный суффикс.
    RenameLoser { loser: NodeId, new_name: NodeName },
    /// Параллельная правка: проигравшая версия материализуется узлом-копией
    /// с НОВЫМ NodeId (Р 1.2).
    ConflictCopy { original: NodeId, copy: NodeId, copy_place: Place },
}

// ============================================================================
// 8. Порты (Д 5.1–5.2). Реализации — адаптеры; фейки для ядра и тестов.
// ============================================================================

/// Порт истории/транспорта (git-адаптер; Д 5.2, Р 2.1).
/// Контракт: параллельное изменение одного объекта ПРЕДЪЯВЛЯЕТСЯ (в
/// PullOutcome), а не разрешается молча — обязательная строка (Д 5.2).
pub trait HistoryPort {
    fn pull(&mut self) -> Result<PullOutcome, PortError>;
    /// После purge метка может исчезнуть из истории — это ШТАТНО:
    /// возвращается PortError::MarkVanished, вызывающий пересчитывает дельту
    /// от ближайшего общего предка либо полным проходом (Р 2.1). Паника
    /// на исчезнувшей метке — баг; обязательный тест-кейс адаптера.
    fn delta_since(&self, mark: &SyncMark) -> Result<RawDelta, PortError>;
    fn read(&self, name: &ObjectName) -> Result<CipherBytes, PortError>;
    /// Старые байты для воскрешений/diff/отката — как были, без переупаковки.
    fn read_at(&self, rev: &Revision, name: &ObjectName) -> Result<CipherBytes, PortError>;
    /// Нормальная форма «один объект — один коммит»; порядок фиксации —
    /// ответственность вызывающего (тело → мета → …; Д 5.4).
    fn commit(&mut self, name: &ObjectName, content: Option<&CipherBytes>) -> Result<(), PortError>;
    /// Вычищение объекта из ВСЕЙ истории (Д 5.2а: обязательная часть passwd;
    /// команда purge для тел не ротируемых секретов). Переписывает хэши
    /// коммитов и требует принудительной публикации; у других реплик после
    /// этого метки исчезают — см. delta_since. Единственные легальные
    /// вызывающие — passwd и команда purge; обычный rm ВСЕГДА обратим
    /// (инвариант «нет невосстановимых ошибок») и делается через commit(None).
    fn purge_from_history(&mut self, name: &ObjectName) -> Result<(), PortError>;
    /// Пуш атомарен для наблюдателей (Д 5.4).
    fn publish(&mut self) -> Result<SyncMark, PortError>;
    fn head(&self) -> Result<SyncMark, PortError>;
}

#[derive(Debug)]
pub enum PullOutcome {
    Clean { head: SyncMark },
    /// Транспортные конфликты; домен превращает их в ConcurrentEdit.
    Conflicted { objects: Vec<ObjectName> },
}

/// Порт упаковщика (gpg-адаптер; Д 5.2а, Р 2.2). VK и слоты — целиком внутри
/// адаптера, домен о них не знает: unlock вычисляет имя слота, разворачивает VK
/// (argon2id); успех развёртывания и есть проверка слота; change_passphrase —
/// цепочка «новый слот → раундтрип-верификация → удаление → purge старого»
/// (Д 5.2а): на уровне дельты D+A, ноль перешифрованных секретов; обрыв цепочки
/// в любой точке оставляет рабочий слот — покрывается её тестом (Д 5.2а).
/// Сверка копий слота — в fsck (Р 2.2). Ротация VK — церемония поверх обоих
/// портов (Р 2.2).
pub trait PackerPort {
    fn unlock(&mut self, passphrase: &Passphrase) -> Result<(), PortError>;
    fn seal_meta(&self, meta: &Meta) -> Result<CipherBytes, PortError>;
    fn open_meta(&self, cipher: &CipherBytes) -> Result<Meta, PortError>;
    fn seal_body(&self, plain: &Plaintext) -> Result<CipherBytes, PortError>;
    fn open_body(&self, cipher: &CipherBytes) -> Result<Plaintext, PortError>;
    fn change_passphrase(&mut self, old: &Passphrase, new: &Passphrase) -> Result<(), PortError>;
    /// Регрессионный счётчик стоимостной модели (Р 3): сценарии обязаны
    /// показывать O(1) распаковок; рост — протёк полный путь или лишняя
    /// классификация.
    fn open_count(&self) -> u64;
}

pub struct Passphrase(pub String);

/// Порт рабочего дерева (материализация/скан; Р 1.1, 1.3, 2.4).
pub trait WorkTreePort {
    fn materialize(&mut self, state: &ValidatedState, packer: &dyn PackerPort) -> Result<(), PortError>;
    /// Скан при свёртке: правки и переезды — по PlainHash (Р 1.3),
    /// неизвестные файлы — по явному правилу «новый секрет / мусор редактора».
    fn scan(&self, table: &Table) -> Result<Vec<LocalChange>, PortError>;
    /// Уборка каталога целиком, включая бэкапы редакторов (Р 1.1).
    fn cleanup(&mut self) -> Result<(), PortError>;
}

/// Без Debug: содержит Plaintext (см. контракт на Plaintext).
pub enum LocalChange {
    Created { place: Place, plain: Plaintext },
    Edited { id: NodeId, plain: Plaintext },
    Moved { id: NodeId, new_place: Place },
    Deleted { id: NodeId },
}

/// Порт персистентности таблицы (Р 1.1: жизненный цикл таблицы = жизненному
/// циклу дерева — таблица существует, пока развёрнуто дерево, и убирается
/// свёрткой вместе с ним; свёрнутое состояние оставляет на диске только
/// репозиторий. Отдельной межсессионной персистентности НЕТ; если появится —
/// шифрованная и с ключёванным PlainHash. Запись — temp+rename, Р 2.3).
pub trait TablePersistence {
    fn load(&self, packer: &dyn PackerPort) -> Result<Option<Table>, PortError>;
    fn store(&mut self, table: &Table, packer: &dyn PackerPort) -> Result<(), PortError>;
}

#[derive(Debug)]
pub enum PortError {
    Transport(String),
    Crypto(String),
    Io(String),
    /// Метка синка исчезла из истории (случился purge) — штатная ситуация,
    /// не авария: пересчитать дельту от общего предка либо полным проходом.
    MarkVanished(SyncMark),
}
