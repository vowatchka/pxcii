# Ревью: этап 1 — Фундамент + чтение форматов

Журнал ведётся по `reviews/PROTOCOL.md`. Строки не удаляются: меняется только статус.
Пустая таблица замечаний означает, что замечаний пока нет.

## Шапка

| Поле | Значение |
|---|---|
| Задача | этап 1 «Фундамент + чтение форматов» |
| В границах | `Cargo.toml` (workspace, `resolver = "2"`, `rust-version`), `rust-toolchain.toml`, `.gitignore`; `pxcii-core/src/{lib,format,decode,error,image,pixels}.rs`, `pxcii-core/src/decode/{dimensions,exif,tests}.rs`; `pxcii-core/Cargo.toml`; `pxcii/` (bin-заглушка); `xtask` (`gen-fixtures`, `check-core-purity`) с тестами; `fixtures/gradient.*` (9 файлов 16×16, генерируются `xtask`); `.github/workflows/ci.yml`; `README.md`, `LICENSE`, `CHANGELOG.md`; проверка наличия `opencode.jsonc`, `.opencode/agents/*`, `.opencode/commands/*` без пересоздания |
| Вне границ | `pxcii-core/src/{encode,convert,resize}.rs` (этап 2); `render/*` (этап 3); CLI-аргументы и файловый I/O (этап 4); растеризация (этап 5); desktop (этап 6); web (этап 7); `pxcii-py` (этап 8); Docker, release-workflow, `NOTICE`, benches (этап 9); правка `pxcii-dev-plan.md`, `pxcii-tz.md`, `AGENTS.md`, `reviews/PROTOCOL.md` |
| Сложность | complex |
| Обоснование | Новое публичное API ядра (`Format`, `decode`, `RgbaImage`, `PxciiError`), ручной парсинг недоверенного ввода (magic bytes, заголовки 9 форматов, EXIF/APP1, TGA-эвристика), 32 файла и ~3,5 тыс. строк в этапном коммите — выше границы medium (6 файлов / 600 строк); срабатывает признак «парсинг недоверенного ввода и границы безопасности» |
| Лимит циклов | 3 |
| Израсходовано циклов | 2 |
| Последний проход | полный, 2026-11-26 |
| Итог | OK |

## Замечания

| ID | Цикл | Severity | Файл:строка | Замечание | Доказательство | Статус | Чем подтверждено |
|---|---|---|---|---|---|---|---|
| F-01 | 1 | minor | `pxcii-dev-plan.md:120`, `.opencode/commands/verify.md:9` | Этапный коммит правит файлы, которые сам журнал объявляет вне границ задачи (`pxcii-dev-plan.md` — источник этапов, `.opencode/commands/*` — конфигурация агентов): изменена формулировка DoD-п.4 и рисков этапа 5, добавлен `$1` в команду ревью. Нужна либо фиксация согласия человека в журнале, либо откат правок | `git show --stat 42b52bd`: `pxcii-dev-plan.md` 21±, `.opencode/commands/verify.md` +3; журнал, строка «Вне границ»; правило `reviews/PROTOCOL.md` §1 «Изменения в диффе за пределами границ задачи — такое же нарушение» | human-rejected | человек, 2026-11-26, правки плана и `verify.md` внесены осознанно в рамках этапа 1 и согласованы |

Для `human-rejected` и `human-deferred` в «Чем подтверждено» пишется «человек, <дата>, <причина>».

## Журнал

| Цикл | Дата | Проход | Команды | Итог |
|---|---|---|---|---|
| 1 | 2026-11-26 | полный | fmt, clippy, test, cov (100% строк), purity — зелёные | ЗАМЕЧАНИЯ 1 |
| 2 | 2026-11-26 | полный | fmt, clippy, test (106 тестов), cov (100% строк по каждому файлу, регионы 97.75% — допустимо п.4 DoD), purity — зелёные; демо-команда проверена, `fixtures/` не изменились. Удалённый CI не запускался: `api.github.com/repos/vowatchka/pxcii/actions/workflows` → `total_count: 0`, приёмка «CI зелёный» подтверждается только локально | OK |
