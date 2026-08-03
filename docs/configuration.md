# Конфигурация mux

`mux` читает TOML-конфиг при запуске. Основной путь:

```text
$XDG_CONFIG_HOME/mux/config.toml
```

Если `XDG_CONFIG_HOME` не задан, используется
`~/.config/mux/config.toml`. Для разового запуска другой файл можно указать
через `MUX_CONFIG=/path/to/config.toml`. Отсутствующий основной конфиг не
является ошибкой: сохраняются встроенные настройки.
Изменения применяются при следующем запуске `mux`; live reload пока нет.

## Пример

```toml
shell = "/bin/zsh"

[sidebar]
visible = true
width = 20
shortcuts = true

[bindings]
new-tab = "ctrl+n"
close-tab = ["ctrl+w", "alt+w"]
split-vertical = "alt+r"
split-horizontal = "alt+d"
pane-left = "ctrl+alt+left"
pane-right = "ctrl+alt+right"
quit = ["alt+q", "f12"]
```

Все поля необязательны. `shell` задаёт исполняемый файл shell для новых pane.
`sidebar.visible` определяет начальную видимость, `sidebar.width` — начальную
ширину; слишком маленькая ширина ограничивается безопасным минимумом.
`sidebar.shortcuts = false` полностью убирает нижние подсказки. Пункт
`Hide/Show shortcuts` в контекстном меню предлагает два режима: `Session`
меняет только текущий запуск, а `Always` применяет выбор сразу и сохраняет это
поле в конфиге, не удаляя комментарии и остальные настройки.

## Бинды

Значением может быть одна строка или массив строк. Указанное действие полностью
заменяет встроенные бинды; пустой массив отключает его:

```toml
[bindings]
close-pane = []
new-tab = ["ctrl+n", "alt+n"]
```

Поддерживаются модификаторы `ctrl`, `alt`, `shift`, `super`, `hyper`, `meta` и
обычные символы. Именованные клавиши: `left`, `right`, `up`, `down`, `enter`,
`escape`, `tab`, `backtab`, `backspace`, `delete`, `insert`, `home`, `end`,
`pageup`, `pagedown`, `space`, `plus`, `minus`, `equals`, `f1` … `f24`.
Модификаторы сопоставляются точно: например, Shift нужно явно записать как
`alt+shift+t`.

Имена действий:

```text
quit                 new-tab              close-tab
next-tab             prev-tab             tab-1 ... tab-9
split-vertical       split-horizontal     close-pane
next-pane            pane-left            pane-right
pane-up              pane-down            toggle-sidebar
sidebar-wider        sidebar-narrower      paste-clipboard
```

Одинаковый пользовательский chord нельзя назначить двум действиям: `mux`
покажет ошибку с именами конфликтующих действий. Подсказки внизу сайдбара
автоматически используют первый бинд каждого настроенного действия.
