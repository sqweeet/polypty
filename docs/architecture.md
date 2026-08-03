# Архитектура mux

Этот документ описывает границы модулей и места расширения. Модульные façade
короткие; реализация каждой возможности лежит в соседних feature-модулях.

## Направление зависимостей

```text
main → runtime → App
App → workspace, session ports, input, render, platform
render::Presenter → workspace snapshot, core geometry, agent model
workspace → TerminalSession port, core geometry, info, agent model
session adapter → Tab
tab → PTY/VT adapters, info, agent
info → agent model
```

- `runtime` владеет временем жизни процесса и host-терминала, создаёт `App`
  и передаёт ей события. Бизнес-правил в event loop нет.
- `App` — верхний оркестратор. Нижние слои не вызывают `App` или `runtime`
  и не читают их состояние напрямую.
- `Workspace` управляет pane с `Box<dyn TerminalSession>`, split-деревом и
  фокусом, а наружу отдаёт read-only `WorkspaceSnapshot`. Конкретный `Tab`
  ничего не знает о layout и сайдбаре.
- `render::Presenter` получает snapshot, владеет render-кешами и пишет
  terminal bytes. Низкоуровневые terminal/sidebar painters не читают `App`
  или `Tab`.
- `agent`, `info`, `input` и `platform` — нижние сервисы. OS-вызовы
  остаются в `platform` и process/PTY-адаптерах, а не в модели интерфейса.

## Композиционные объекты

| Объект | Ответственность и состав |
|---|---|
| `MuxRuntime` | Устанавливает `ShutdownLatch`, входит в `HostTerminal` через RAII и запускает `EventLoop` с `App`, stdout и `ResizeWatcher`. |
| `App` | Содержит `WorkspaceBook`, `Viewport`, `FrameScheduler`, render-`Presenter`, заменяемые `Clipboard` и `SessionFactory`. Модули `session`, `interaction`, `polling`, `resize`, `draw` и `lifecycle` реализуют его façade. |
| `Workspace` | Один пункт сайдбара: `PaneStore`, `SplitTree` и `FocusModel`. Каждая pane содержит одну `TerminalSession`; `snapshot` проецирует состояние для renderer без передачи владения. |
| `Tab` | Стандартная PTY-реализация `TerminalSession`: `PtyTransport`, `TerminalEmulator`, `SessionMetadata` и `AgentTracker`. Создаётся через `PtySessionFactory`. |
| `render` | Façade над `Presenter`, geometry/frame/divider, terminal cell/frame/pen/painter/cache и sidebar model/card/viewport/frame/painter/cache. `Presenter` хранит per-workspace renderer с `TermCache` каждой pane и `SidebarPresentation` с cache/map/animation. |
| `agent` | Модель `AgentKind/State/Status`, каталог профилей, identity, анализ свежих screen/title evidence и rollup pane-статусов. Владение временем и evidence остаётся в `Tab::AgentTracker`. |

`info` отдельно собирает OSC и foreground-process metadata и составляет
`TabInfo`; это вход для `Tab`, а не второй владелец terminal state.
`session` задаёт порты `TerminalSession`/`SessionFactory`, отделяя
оркестрацию и workspace от создания реального PTY.

## Как расширять

### Новый профиль coding agent

1. Добавить вариант в `agent/model.rs::AgentKind`.
2. Добавить ровно один `AgentProfile` в `agent/catalog.rs::PROFILES`: label,
   имена бинарников, package markers и признак `explicit_screen_state`.
3. Если у агента есть надёжные собственные сигналы, добавить их в
   `agent/detection/screen.rs` или `title.rs`. Общий activity fallback
   менять только при изменении общей причинной модели.
4. Дополнить identity- и detection-тесты в `agent/tests/`; при изменении
   объединения pane — также rollup-тесты.

Для каждого `AgentKind` профиль обязателен: `AgentKind::label()` разрешается
через каталог.

### Ввод

- Новая команда mux: вариант `input::Action`, binding в `input/keymap.rs`
  и обработчик в `app/interaction/`.
- Новая terminal sequence: соответствующий модуль `input/keyboard/` или
  `input/mouse/`; неизвестная комбинация должна оставаться `Action::Forward`.
- Проверки добавляются в `input/tests/`, а orchestration — рядом с тестами
  `app/interaction`.

### Рендерер

- Terminal grid: `render/terminal/` — преобразование cell, построение кадра,
  cursor/style pen и diff-painter.
- Sidebar: `render/sidebar/` — model/card/viewport, palette/glint/badge,
  hit map, animation/presentation, frame и painter.
- Workspace composition: `render/presenter.rs` и `render/workspace/` —
  кеши по workspace/pane, синхронизация snapshot и порядок рисования.
- Geometry, synchronized frame и split dividers находятся непосредственно в
  `core/geometry.rs` и `render/`. Новое внешнее API реэкспортируется через
  `render.rs`.

Изменение геометрии или смысла кешированной ячейки должно сопровождаться
корректной invalidation. Активная pane рисуется последней и одна владеет
hardware cursor.

### Platform service

Следует шаблону `platform::clipboard`: небольшой trait, системная реализация и
инъекция в `App` (с тестовой реализацией). Команды ОС и выбор backend остаются
в `platform/<service>/`; прикладной код зависит от trait, а не от shell
command или конкретной ОС. Для сервисов terminal lifecycle аналогичная граница
уже задана `session::TerminalSession` и `SessionFactory`.

## Инварианты

- Каждому leaf `SplitTree` соответствует pane в `PaneStore`; активный ID
  `FocusModel` указывает на одну из них. Split и remove синхронно обновляют
  доменные структуры.
- `Presenter` индексирует render state по workspace/pane ID, удаляет его
  вместе с workspace и синхронизирует видимые кеши из каждого snapshot.
- `App` не хранит cache/map/animation рисования: весь изменяемый paint state
  принадлежит `Presenter`, а orchestration передаёт ему domain snapshots.
- Одна pane владеет одной `TerminalSession`; стандартный `Tab` владеет одним
  PTY, одним VT parser, metadata и agent evidence. PTY output сначала проходит
  parser, затем становится доступен render.
- Layout не мутирует split-дерево: compact viewport может скрыть ветви, но
  увеличение окна восстанавливает исходную структуру.
- Неактивные cursor скрыты; активная pane рисуется последней. После clear,
  resize и смены геометрии соответствующие render-кеши инвалидируются.
- Agent evidence относится только к текущему foreground agent; при смене
  identity оно сбрасывается, а OSC title учитывается лишь в activity window.
  Rollup сохраняет приоритет `blocked > working > ready`.
- `HostTerminal` обязан восстановить terminal modes при обычном выходе,
  ошибке и сигнале завершения.

## Тесты и архитектурный gate

Тесты располагаются рядом с feature: в `tests.rs`, `tests/mod.rs` или
подкаталоге `tests/`. Основные локальные проверки:

```bash
./scripts/check-architecture.sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

`check-architecture.sh` считает физические строки каждого `.rs` в `src/`
и `tests/` и отклоняет файл длиннее 150 строк. При росте feature нужно
выделять модель, сервис, painter и тесты в отдельные модули, сохраняя короткий
façade. CI выполняет gate, fmt, clippy, тесты на Linux/macOS и release build.
