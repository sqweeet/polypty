# CLI-сессии mux

Интерактивный процесс `mux` владеет PTY и одновременно обслуживает приватный
Unix socket. Любой локальный процесс того же пользователя может вызвать
`mux <command>`: короткоживущий CLI-клиент отправит запрос живому UI и выведет
ответ. Команды можно безопасно запускать из Codex, Claude Code, Pi или shell.

## Команды

| Команда | Алиасы | Результат |
|---|---|---|
| `list-sessions` | `ls`, `sessions` | PID, число вкладок и активный номер |
| `list-tabs` | `list-windows`, `l` | вкладки, pane IDs, cwd и agent status |
| `new-tab` | `new-window`, `neww` | новая активная вкладка |
| `select-tab TARGET` | `select-window`, `selectw` | переключение UI на вкладку |
| `close-tab TARGET` | `kill-window`, `killw` | закрытие вкладки |
| `capture-pane` | `capturep` | текст текущей видимой VT-сетки |
| `send-keys` | `send` | запись текста в PTY |
| `ping` | — | проверка доступности event loop |

Для всех control-команд доступен `--json`. Полная встроенная справка:
`mux --help`.

## Targets

Вкладку можно указать как one-based номер (`1`, `2`), активную вкладку
(`active` или `.`) либо стабильный ID (`@17`). Pane задаётся через `-p 21` или
`-p %21`. Примеры:

```bash
mux capture-pane --tab @17 --pane %21
mux send-keys -t 2 -p 21 --enter -- 'git status --short'
mux kill-window -t @17
```

`capture-pane` возвращает текущий экран без scrollback, убирает правый padding
строк и пустые строки в конце. `send-keys` объединяет текстовые аргументы пробелами;
`--enter` дописывает terminal Return (`\r`). Для текста, начинающегося с `-`,
используйте разделитель `--`.

## Контекст внутри pane

Каждая shell наследует `MUX_SESSION`, `MUX_TAB`, `MUX_PANE` и `MUX_SOCKET`.
Поэтому `capture-pane` и `send-keys` без targets из pane адресуют именно её,
даже если пользователь успел переключить UI на другую вкладку. Внешний клиент
без этих переменных работает с активной pane.

Путь сокета выбирается из `MUX_SOCKET`, затем
`$XDG_RUNTIME_DIR/mux.control.sock`, иначе `/tmp/mux-UID.sock`. Сервер создаёт
socket с mode `0600`, не заменяет чужой или не-socket path и удаляет свой path
при завершении.

## Граница текущей версии

Это attached session server: он доступен, пока жив интерактивный процесс mux.
Закрытие host-терминала или сигнал завершения останавливает дочерние PTY.
Полноценный daemon с detach/reattach потребует переноса владения PTY из UI в
отдельный процесс и является следующим отдельным этапом.
