# polypty

[![CI](https://github.com/sqweeet/polypty/actions/workflows/ci.yml/badge.svg)](https://github.com/sqweeet/polypty/actions/workflows/ci.yml)

<img width="1013" height="713" alt="image" src="https://github.com/user-attachments/assets/bddc9fe7-1615-44be-b068-cc4ef5ec2f16" />

Минимальный terminal multiplexer на Rust для Linux/Unix: вкладки в сайдбаре,
вложенные split panes и отдельный PTY для каждой pane. Область терминала
рендерится напрямую из VT-сетки, поэтому полноэкранные TUI — редакторы,
мониторы и интерфейсы агентов — сохраняют цвета, wide/combining characters,
alternate screen, mouse tracking и настоящий курсор.

Название `polypty` складывается из `poly` + `PTY`: несколько псевдотерминалов
собраны в одном многопанельном рабочем пространстве.

## Запуск

Из корня репозитория:

```bash
cargo run --release
```

Либо установите бинарник из crates.io и запускайте его как `polypty`:

```bash
cargo install polypty
polypty
```

Одновременно для одного пользователя работает только один интерактивный
экземпляр `polypty`. Повторный запуск без команды завершится с понятной ошибкой,
но `polypty <command>` работает как control-клиент уже запущенного экземпляра.
Instance-lock автоматически освобождается при выходе или аварийном завершении.

Нужен актуальный Rust toolchain и терминал с поддержкой ANSI/alternate screen.
Для чтения буфера обмена установите хотя бы одну внешнюю утилиту:

- Wayland: `wl-paste` из пакета `wl-clipboard`;
- X11: `xclip` или `xsel`.

Без них сам polypty работает, но `Ctrl+Shift+V` и fallback-вставка средней кнопкой
не смогут прочитать системный clipboard/PRIMARY selection.

## Конфигурация

Настройки читаются из `~/.config/polypty/config.toml` (или
`$XDG_CONFIG_HOME/polypty/config.toml`). Можно менять бинды, начальную ширину и
видимость сайдбара, а также shell для новых pane. Неуказанные значения сохраняют
встроенные defaults; подсказки в сайдбаре следуют настроенным биндам.

```toml
shell = "/bin/zsh"

[sidebar]
visible = true
width = 20
shortcuts = true

[bindings]
new-tab = "ctrl+n"
quit = ["alt+q", "f12"]
close-pane = []
```

Полный формат, имена действий и клавиш: [docs/configuration.md](docs/configuration.md).

## CLI и агентские сессии

Работающий polypty поднимает приватный Unix control socket. Поэтому человек или
coding agent из любой pane может управлять вкладками и читать их текущий экран,
не перехватывая интерактивный терминал:

```bash
polypty list-sessions
polypty list-tabs --json
polypty new-window
polypty capture-pane -t 2
polypty send-keys -t 2 --enter -- 'cargo test'
polypty select-window 2
```

Доступны команды `list-sessions` (`ls`), `list-tabs` (`list-windows`),
`new-tab` (`new-window`), `select-tab`, `close-tab`, `capture-pane`,
`send-keys` и `ping`. Вкладка адресуется номером, `active` или стабильным
`@ID`; pane — числом или `%ID`. Флаг `--json` даёт стабильный машинный ответ.
Без `-t`/`-p` команда из pane использует её `POLYPTY_TAB`/`POLYPTY_PANE`, а внешний
клиент — активную pane интерфейса.

Control server живёт вместе с открытым интерактивным UI. Закрытие host-терминала
пока завершает PTY; daemon detach/reattach в эту версию не входит. Полный CLI,
формат targets и переменные окружения: [docs/sessions.md](docs/sessions.md).

## Управление

| Клавиши | Действие |
|---|---|
| `Alt+t` | создать вкладку |
| `Alt+w` | закрыть активную вкладку со всеми её pane |
| `Alt+]`, `Alt+n`, `Alt+Right` | следующая вкладка |
| `Alt+[`, `Alt+p`, `Alt+Left` | предыдущая вкладка |
| `Alt+1` … `Alt+9` | перейти к вкладке по номеру |
| `Alt+v` | разделить активную pane вправо |
| `Alt+s` | разделить активную pane вниз |
| `Alt+x` | закрыть активную pane; если она последняя — закрыть вкладку |
| `Alt+o` | перейти к следующей pane |
| `Alt+h/j/k/l` | сфокусировать pane слева/снизу/сверху/справа |
| `Ctrl+Alt+Arrow` | сфокусировать pane в направлении стрелки |
| `Alt+b` | показать или скрыть сайдбар |
| `Alt+=`, `Alt++` | расширить сайдбар |
| `Alt+-`, `Alt+_` | сузить сайдбар |
| `Ctrl+Shift+V` | вставить системный clipboard в активную pane |
| `Alt+q`, `Ctrl+Alt+q` | выйти из polypty |

Остальные клавиши кодируются как terminal sequences и передаются активному
дочернему PTY. Вставка учитывает bracketed-paste mode дочернего приложения.
Выход и попытка закрыть последнюю вкладку сначала показывают подтверждение с
кнопками `Cancel` и `Exit`. По умолчанию выбран безопасный `Cancel`; доступны
стрелки/Tab + Enter, `y`/`n`, Escape и мышь.

### Мышь

- Вкладки мягко меняют яркость на hover/press и при переключении. Left-click
  срабатывает при отпускании на той же вкладке; drag наружу отменяет выбор.
  Scroll над сайдбаром листает вкладки.
- Right-click в сайдбаре открывает компактное меню `New tab` и
  `Hide/Show shortcuts`; над карточкой также появляется `Close tab`. В области
  pane правый клик по-прежнему принадлежит TUI.
- `Hide/Show shortcuts` предлагает `Session` для временного изменения или
  `Always`, чтобы сразу применить и сохранить настройку в конфиге.
- Middle-click по карточке в сайдбаре закрывает эту вкладку.
- Перетаскивание правого края сайдбара меняет его ширину без скачка в точке
  захвата. Кнопки активируются при отпускании; drag наружу отменяет нажатие.
- Click по pane фокусирует её. Если дочерний TUI включил mouse tracking,
  события передаются ему в локальных координатах pane.
- Middle-click внутри pane передаётся TUI с mouse tracking. Если tracking не
  включён, polypty вставляет PRIMARY selection (и использует обычный clipboard как
  fallback).
- `Shift` оставляет выделение текста host-терминалу.

## Вкладки, сайдбар и splits

Каждая вкладка — отдельный workspace. Внутри неё можно строить вложенное дерево
вертикальных и горизонтальных splits; у каждой pane собственные shell, PTY,
VT-состояние и курсор. Сайдбар показывает процесс или OSC title активной pane
каждой вкладки и сокращённый текущий путь. Активная карточка остаётся видимой
даже при низком окне.

При слишком маленьком viewport polypty временно показывает только ветку с активной
pane, не сжимая скрытые TUI до разрушительного размера `1x1`. После увеличения
окна дерево splits и рабочие размеры pane восстанавливаются.

## Статусы coding agents

На Linux polypty распознаёт coding agent в foreground process group каждой pane и
заменяет первую строку карточки компактным живым статусом:

| Вид | Состояние |
|---|---|
| `codex` + мягкий белый glint обеих строк | агент выполняет задачу |
| `codex` + жёлтый `!` справа | агент ждёт подтверждения или ответа |
| `codex` + маленький зелёный `✓` справа | агент готов к следующей команде |

Во время работы один широкий мягкий нейтрально-белый блик плавно проходит по фону
обеих строк без наклона; текст, путь и геометрия не анимируются.
После примерно четырёхсекундного прохода следует спокойная двухсекундная пауза.
У каждой карточки собственная фаза от момента перехода в `working`, поэтому
агенты, запущенные в разное время, не двигаются строем. В состоянии готовности
справа появляется компактный зелёный `✓`. Если во вкладке несколько split panes
с одинаковым агентом, карточка объединяет их как `codex ×2`; смешанный набор
показывается через главный агент и число остальных, например `claude+1`.
Наверх поднимается самый важный статус: `blocked` → `working` → `ready`, включая
неактивные pane. В состоянии `blocked` справа появляется компактный жёлтый `!`.

Идентификация поддерживает Codex, Claude Code, OpenCode, Gemini CLI, Cursor
Agent, GitHub Copilot, Kimi, Amp, Pi, Devin, Droid, Kiro и Grok. Для Codex,
Claude Code и OpenCode используются известные сигналы live-screen и OSC title;
для остальных состояние определяется best-effort по живому экрану и активности
PTY. Неизвестные процессы продолжают работать как обычно без agent badge.

Детектор анализирует только текущий экран подтверждённого foreground-процесса.
Echo обычного ввода, пустой Enter, terminal-control traffic и перерисовка после
resize не считаются работой. Submit, live footer и свежий OSC title учитываются
отдельно; evidence полностью сбрасывается при смене foreground-процесса. Поэтому
фоновый helper, введённые пользователем слова-маркеры или старая approval-фраза
из scrollback не должны запускать glint. Это только индикация — polypty никогда не
отвечает агенту автоматически. Подход к карточкам и приоритетам вдохновлён
[Herdr](https://github.com/herdrdev/herdr).

## Терминальная совместимость и устойчивость

Каждый дочерний процесс получает terminal capabilities и контекст polypty:

```text
TERM=xterm-256color
COLORTERM=truecolor
POLYPTY=1
POLYPTY_SESSION=main
POLYPTY_TAB=<stable tab id>
POLYPTY_PANE=<stable pane id>
POLYPTY_SOCKET=<absolute control socket path>
```

Они не наследуются от host-терминала: дочерние TUI видят только возможности,
которые polypty действительно эмулирует и кодирует.

Путь вывода рассчитан на долго работающие и активно печатающие процессы:

- VT parser хранит alternate screen, RGB/256-color, стили, Unicode-геометрию,
  cursor modes и mouse modes;
- очередь чтения PTY ограничена; при заполнении backpressure доходит до PTY и
  дочернего процесса вместо неограниченного роста памяти;
- запись клавиш и terminal-query replies идёт через отдельную ограниченную
  очередь каждой pane: дочерний процесс, переставший читать stdin, не может
  заморозить остальные вкладки, resize или выход из polypty;
- поток вывода обрабатывается с byte budget и собирается в цельные кадры, чтобы
  вложенные TUI не мерцали промежуточными состояниями;
- рендер перерисовывает только изменившиеся ячейки и строки сайдбара.

Во время drag-resize polypty сразу следует последней геометрии host-терминала, но
объединяет пачку промежуточных событий в один финальный resize дочерних PTY.
После каждого наблюдаемого resize устаревшие render caches инвалидируются, а
после финального `SIGWINCH` даётся короткое время на новый кадр TUI. Это не даёт
табам, сайдбару и split-разделителям остаться в геометрии промежуточного окна.

При выходе, а на Unix также при `SIGTERM`, `SIGHUP`, `SIGINT` и `SIGQUIT`, polypty
завершает дочерние процессы и восстанавливает raw mode, alternate screen,
mouse capture, bracketed paste, курсор и autowrap host-терминала.
Если окно терминала исчезло без `SIGHUP`, polypty дополнительно обнаруживает hangup
или потерю host TTY и сам завершает PTY, control socket и instance-lock.

## Структура кода

Подробная карта зависимостей, композиционных объектов, инвариантов и точек
расширения находится в [docs/architecture.md](docs/architecture.md).

- `runtime/` управляет host-терминалом, сигналами и event loop;
- `control/` содержит JSON-протокол, Unix socket server и tmux-подобный CLI;
- `app/` оркестрирует workspaces, ввод, resize, sidebar и render-`Presenter`;
- `workspace/` владеет terminal sessions, split-деревом и фокусом, отдавая
  read-only snapshot для кадра;
- `session/` задаёт заменяемые session/factory ports, а `tab/` реализует их
  через PTY, VT parser, metadata и agent evidence;
- `render/` владеет diff-кешами и строит terminal grid, sidebar и dividers;
- `core/` содержит общие value types геометрии без инфраструктурных зависимостей;
- `agent/`, `info/`, `input/` и `platform/` содержат нижние модели и
  OS-сервисы.

CI ограничивает каждый Rust-файл, включая тесты, 150 физическими строками.

## License

[MIT](LICENSE)
