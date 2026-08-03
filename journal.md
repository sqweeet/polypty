# mux improvement journal

## 2026-08-03 — Round 1: baseline and failure inventory

### Verified baseline

- `cargo test`: 27 passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- Interactive startup and `Alt+q` shutdown work.

### Confirmed defects to address

- Unbounded PTY output queue and unbounded drain can starve input/render or exhaust memory.
- Closing the final workspace with middle-click leaves an invalid empty app state.
- `Shift+Tab` is dropped and F1 is encoded as a Home sequence.
- Tiny split layouts can leave hidden panes permanently dirty and continuously redraw.
- Focusing a pane hidden by a tiny layout can expose stale 1x1 PTY geometry.
- Renderer and `vt100` disagree on Unicode cell widths; adjacent cells can disappear.
- OSC 7 percent-decoding corrupts non-ASCII paths.
- Terminal restoration is not panic/setup-error safe.
- README and advertised `TERM` behavior do not match the implementation.

### Next vectors

1. Add focused regression tests and fix PTY/input/terminal lifecycle.
2. Make resize/layout state converge for visible and hidden panes.
3. Align renderer cell widths with the emulator and exercise agent-like frames.
4. Run interactive resize/output stress smoke tests and update documentation.

## 2026-08-03 — Round 2: core stabilization

### Implemented

- Replaced the unbounded PTY channel with a bounded ~256 KiB queue per pane.
- Limited parsing to 32 KiB per pane per event-loop tick while preserving chunk tails exactly.
- Deferred reap until child exit, reader EOF, and full queued-output drain.
- Corrected BackTab and xterm F1-F12 encoding.
- Fixed child capability policy to `TERM=xterm-256color` plus truecolor.
- Added an unwind-safe terminal guard for raw mode, alternate screen, mouse,
  bracketed paste, synchronized output, autowrap, and cursor restoration.
- Preserved hidden split-pane geometry during tiny resizes and excluded hidden
  dirty panes from draw decisions.
- Synchronized PTY/parser geometry immediately after pane focus, tab switch,
  split, close, and split collapse.
- Made final middle-click close request quit without creating an invalid empty app.
- Made sidebar card layout keep the active tab visible at every tested height.
- Made cell rendering trust the VT grid width, preserving adjacent text such as `☰X`.
- Fixed UTF-8 OSC 7 paths and stateful/sanitized OSC titles.
- Overrode crossterm's `NO_COLOR` suppression for emulated child cells; colors
  are terminal data here and must not be discarded.

### Evidence

- `cargo test`: 43 passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt -- --check`: passed.
- Sustained 512 KiB channel test proves byte-exact budgeted draining.
- Agent-like alternate-screen/RGB/wide/combining/partial-redraw/resize test passes.

### Next vectors

1. Upgrade the VT parser to the maintained 0.16 line, including its resize cursor fix and dim support.
2. Run a real tmux-driven resize/split/tab/output stress scenario.
3. Synchronize README and packaging, then repeat the completion audit.

## 2026-08-03 — Round 3: maintained VT stack and real-TUI stress

### Implemented

- Upgraded to `vt100 0.16.2`, `portable-pty 0.9.0`, and `crossterm 0.29.0`;
  the dependency tree has no duplicate package versions.
- Added dim rendering and preserved SGR reverse as an attribute for both
  default and explicit colors (without losing or double-inverting it).
- Added safe child-terminal replies for OSC 10/11/12 color queries, ANSI
  status/cursor reports, and primary/secondary device attributes. Unknown OSC,
  including clipboard queries, is not forwarded to the host terminal.
- Invalidated every host render cache on every observed resize event, including
  bursts that return to the original dimensions. This fixes physical-cell
  reflow corruption seen under tmux with a real Codex TUI.
- Rewrote the README to match the actual keys, mouse behavior, clipboard
  helpers, terminal capability policy, backpressure, and resize model; added
  the declared MIT license.

### Evidence

- Debug and release test suites: 47 passed, 0 failed.
- Strict `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt -- --check`: passed; `cargo tree -d`: empty.
- Neovim 0.12.4 starts inside mux without E1568, renders its alternate-screen
  RGB frame, and redraws after `42x8 -> 160x45 -> 55x12 -> 120x35`.
- Eight live workspaces keep the active sidebar card visible at heights 5 and
  1; nested vertical/horizontal panes accept input after a `25x2 -> 140x40`
  round trip.
- Live SGR 7 output is present in the host capture. Under continuous `yes`
  output, `Alt+q` closed the isolated stress session in 68 ms.

### Next vectors

1. Repeat the original real-Codex resize reproduction on the final binary.
2. Complete an independent source audit and close only if it finds no remaining
   high/medium regression in the target rendering and resize paths.

## 2026-08-03 — Round 4: adversarial audit and shutdown closure

### Implemented

- Moved child PTY writes to a per-pane writer thread with a bounded 256-message
  input queue. Automatic terminal replies allow only one buffer in flight, so
  a child that requests replies without reading stdin cannot block the main
  event loop or grow memory without bound.
- Installed Unix shutdown latches for `SIGTERM`, `SIGHUP`, `SIGINT`, and
  `SIGQUIT` before entering raw mode. Signal exits now kill children and pass
  through the same terminal-guard restoration as keyboard exit.
- Repainted sidebar metadata when the active pane of a background workspace is
  reaped; its process/title can no longer remain stale indefinitely.
- Made sidebar wrapping and padding grapheme-aware. Combining sequences and ZWJ
  emoji remain atomic and no longer trigger false ellipses.

### Evidence

- Debug and release suites: 50 passed, 0 failed; strict clippy and rustfmt are
  clean; `cargo tree -d` remains empty.
- A raw/no-echo child emitted 100,000 OSC 11 queries and never read stdin. mux
  still opened another tab and `Alt+q` exited the isolated session in 117 ms.
- Sending `SIGTERM` during the same adversarial flood exited in 73 ms; tmux
  reported `alternate_on=0`, `mouse_any_flag=0`, and `mouse_sgr_flag=0`.
  Separate live checks for HUP and INT produced the same restored state.
- Neovim on the asynchronous input path starts without E1568 and converges
  after the full resize burst; a raw query client receives
  `CSI 5 n -> CSI 0 n`.
- A background split-pane exit changed its sidebar card from `sleep` to the
  surviving shell without tab switching or another resize.
- An independent final rerun measured 70 ms for `Alt+q` and 68 ms for
  `SIGTERM` under the same query flood, restored host modes to `0/0/0`, and
  found no remaining high/medium blocker in the target paths.

## 2026-08-03 — Round 5: CI and live agent visibility

### Implemented

- Added GitHub Actions validation on pushes to `main` and pull requests: fmt,
  strict clippy, tests, and release builds across Ubuntu and macOS.
- Added per-pane coding-agent identification from the terminal's real
  foreground process group. Background jobs and arbitrary runtime arguments
  cannot masquerade as an agent.
- Added live `ready`, `working`, and `blocked` classification from the current
  VT screen, fresh OSC title, and PTY activity. Old scrollback and OSC titles
  from a previous foreground job are excluded.
- Aggregated split-pane state into each workspace card with priority
  `blocked > working > ready`, so an inactive blocked pane remains visible.
- Added restrained green/yellow/red status rows without increasing the
  two-line sidebar card height; semantic state is included in redraw caching.

### Evidence

- `cargo test`: 60 passed, 0 failed.
- `cargo fmt --all -- --check`, strict clippy, and release build pass.
- Foreground-group tests prove a newer background agent cannot replace the
  foreground Codex; runtime payload and `tmux` false positives are covered.
- A live tmux-driven PTY check observed `● codex · working`,
  `● codex · blocked`, and `○ codex · ready` in mux's real sidebar.

## 2026-08-03 — Round 6: restrained working glint

### Implemented

- Removed status circles and the redundant `working` label. A working card now
  shows only the agent name; `ready` and the important `blocked` state remain
  explicit suffixes.
- Replaced the working text color with one low-contrast warm background band
  moving across the primary row every 120 ms. Text, path, and layout stay
  stable.
- Quantized animation from a monotonic epoch, so delayed frames skip forward
  without timer drift. Off-screen working cards do not schedule animation.
- Kept glint frames sidebar-only: they repaint cached background spans and
  restore the active PTY cursor without rebuilding the terminal cell grid.

### Evidence

- `cargo test`: 64 passed, 0 failed; strict clippy, rustfmt, and release build
  pass.
- Cache tests prove identical steps emit nothing, a new step changes only the
  working primary row, and Unicode spans retain exact display width.
- A live tmux check showed the stable `codex` label while two captures differed
  in background SGR only. An independent audit found no animation, cursor,
  resize, cache, or performance blocker.

## 2026-08-03 — Round 7: full-card glint and READY badge

### Implemented

- Extended the working glint across both the agent-name and path rows with one
  shared phase, so the two-line tab reads as a single card.
- Added half-cell color interpolation at 80 ms per frame. The wider muted
  warm-neutral highlight now completes a default-width pass in about 4.16 s
  without abrupt per-cell color jumps.
- Removed the inline ready suffix. Ready cards keep the agent name on the left
  and render a right-aligned green `READY` badge with dark text.
- Moved sidebar foreground color into cached spans so the badge and ordinary
  text can use different foreground/background pairs without style leakage.

### Evidence

- `cargo test`: 66 passed, 0 failed; strict clippy, rustfmt, and release build
  pass.
- Consecutive-frame RGB deltas are bounded, both working rows change together,
  and READY badges preserve exact widths from 1 through 18 columns.
- Live tmux captures confirmed background changes on both rows and rendered
  `codex       READY` with the expected green badge SGR.
- Independent audit found no color-math, cache, narrow-width, cursor, resize,
  or animation-performance blocker.

## 2026-08-03 — Round 8: compact ready mark and cool glint

### Implemented

- Replaced the seven-cell `READY` word badge with a quiet three-cell ` ✓ `
  mark aligned to the card's right edge. One- and two-cell sidebars keep an
  exact-width fallback without dropping the symbol.
- Muted the ready colors to a dark green background with a light glyph, keeping
  the mark legible without competing with the explicit red blocked state.
- Shifted the working highlight from warm neutral to restrained steel blue
  while preserving the two-row phase, 80 ms cadence, and smooth interpolation.

### Evidence

- `cargo test`: 66 passed, 0 failed; strict clippy, rustfmt, release build, and
  `git diff --check` pass.
- Width tests cover the exact `✓`, ` ✓`, and ` ✓ ` narrow forms; color tests
  pin the new ready and glint palettes and bound consecutive-frame RGB deltas.
- A live tmux capture rendered `codex           ✓` and confirmed the expected
  foreground/background SGR values. Independent audit reported no findings.

## 2026-08-03 — Round 9: neutral glint and split-agent counts

### Implemented

- Removed the blue cast from the working animation. Both active and inactive
  highlights now blend through equal RGB channels for a soft neutral-white
  sheen across the existing two-row card.
- Extended the workspace agent rollup with the number of represented panes.
  Two matching split panes render as `codex ×2`; mixed kinds keep the dominant
  agent and append the number of other panes, such as `claude+1`.
- Kept the existing state priority across the aggregate. Any blocked pane wins,
  otherwise working wins over ready, so the ready mark appears only when every
  detected agent pane is ready.
- Preserved critical blocked text on narrow sidebars by progressively reducing
  it to `… · blocked`, `blocked`, and finally `!` instead of clipping its end.

### Evidence

- `cargo test`: 67 passed, 0 failed; strict clippy, rustfmt, release build, and
  `git diff --check` pass.
- Rollup tests cover matching and mixed kinds, pane counts, priority, and the
  active-pane tie. Renderer tests cover split labels and narrow blocked forms.
- A live tmux smoke created two real split panes, launched Codex-shaped working
  processes in both, rendered `codex ×2`, and verified neutral glint spans on
  both card rows. Independent audit reported no findings.

## 2026-08-03 — Round 10: subsystem split and causal agent activity

### Implemented

- Split the former monolithic render, app, metadata, and workspace code into
  responsibility-owned modules: sidebar/terminal rendering, draw/interaction,
  OSC/process probes, split-tree layout, sidebar animation, and PTY activity.
- Replaced the shared glint clock with an immutable workspace identity and a
  per-card `Working` epoch. Later agent starts now receive a genuinely different
  phase; focus, split changes, metadata refreshes, and resize preserve it.
- Refined the neutral two-row sweep into an approximately four-second pass plus
  a collapsed two-second rest. Invisible, offscreen, narrow, and visually empty
  frames do not keep the redraw loop active.
- Replaced raw “recent PTY bytes = work” logic with causal evidence. Editing,
  empty Enter, bracketed paste, control-only output, and SIGWINCH redraws cannot
  promote an idle Codex/Claude/OpenCode card. A non-empty submit, anchored live
  status, bounded fresh OSC signal, or qualified activity drives transitions.
- Scoped footer markers away from wrapped composer text, kept working state
  while a follow-up is edited, and reset screen/activity/title evidence whenever
  the foreground agent changes.

### Evidence

- `cargo test --locked --all-features`: 93 passed, 0 failed; strict clippy,
  rustfmt, `git diff --check`, and locked release build pass.
- Live Codex 0.146.0 smoke verified that marker words typed into the composer,
  empty Enter, and repeated sidebar resize remain `ready`; `!sleep 3` starts the
  glint, keeps it through resize, and returns to the compact `✓` afterward.
- A two-workspace PTY smoke started Codex-shaped workers at different times and
  captured different neutral highlight positions while both rows of each card
  shared one phase.
- Independent detector, animation, and architecture audits found no remaining
  blocker after the cross-process OSC and erased-draft regressions were fixed.
