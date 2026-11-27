# Ревью: этап 3 — Ядро рендера (псевдографика)

Журнал ведётся по `reviews/PROTOCOL.md`. Строки не удаляются: меняется только статус.
Пустая таблица замечаний означает, что замечаний пока нет.

## Шапка

| Поле | Значение |
|---|---|
| Задача | этап 3 «Ядро рендера (псевдографика)» |
| В границах | `pxcii-core/src/render/*`: `options.rs` (`RenderOptions`, `ColorMode`, `Ramp`, `Dither`), `grid.rs` (`Cell`, `Grid`), `resize.rs` (приведение к `cols`×`rows`), `luma.rs` (sRGB↔linear, яркость), `ramp.rs` (Ascii/Blocks/Braille), `dither.rs` (Floyd–Steinberg, Bayer 8×8), `ansi.rs` (`to_ansi`), `text.rs` (`to_text`), `mod.rs` (`render_bytes`), их тесты; экспорт в `pxcii-core/src/lib.rs`; golden-гриды `fixtures/golden/grids/*.txt` и их генерация в `xtask gen-fixtures`; `pxcii-core/examples/render.rs` (демо этапа); `README.md`, `CHANGELOG.md` |
| Вне границ | CLI-аргументы и файловый I/O в `pxcii/` (этап 4); растеризация в изображение (этап 5); desktop (этап 6); web (этап 7); `pxcii-py` (этап 8); релиз-обвязка (этап 9); изменения в `decode`/`encode`/`convert`/`format` этапов 1–2; новые форматы и рампа сверх перечисленных в плане; правка `pxcii-dev-plan.md`, `pxcii-tz.md`, `AGENTS.md`, `reviews/PROTOCOL.md` |
| Сложность | complex |
| Обоснование | Численное ядро рендера (box-фильтр, яркость в линейном свете, дизеринг Floyd–Steinberg и Bayer 8×8), новое публичное API (`render_bytes`, `Grid`, `Cell`, `RenderOptions`, `ColorMode`, `Ramp`, `Dither`) и новые golden-файлы, закрепляющие детерминизм вывода; `render/*` — 10 файлов, 1316 строк суммарно с демо, плюс правки `xtask`, `README`, `CHANGELOG` — выше границы medium. Срабатывают признаки «численное ядро (рендер, дизеринг)» и «изменение golden-детерминизма» |
| Лимит циклов | 3 |
| Израсходовано циклов | 2 |
| Последний проход | полный, 2026-11-27 |
| Итог | OK |

## Замечания

| ID | Цикл | Severity | Файл:строка | Замечание | Доказательство | Статус | Чем подтверждено |
|---|---|---|---|---|---|---|---|
| F-01 | 1 | minor | `pxcii-core/src/render/options.rs:10,24`, `pxcii-core/src/render/resize.rs:8`, `README.md:71` | `cell_aspect` описан как «высота знакоместа / ширина», хотя формула `rows = cols*(h/w)*cell_aspect` верна только при `cell_aspect` = ширина/высота знакоместа | Арифметика: при `a` = высота/ширина формула дала бы `rows = cols*(h/w)/(h_cell/w_cell)`; проверка на golden: `fixtures/golden/grids/ascii-truecolor.txt` — 8×4 знакоместа для квадрата 16×16 при `a = 0.5`, что на терминале 2:1 даёт квадрат | verified | проверено в коде 2026-11-27: `options.rs:10,24`, `resize.rs:8-9`, `README.md:71` — «ширина/высота»; при такой трактовке `rows = cols*(h/w)*cell_aspect` даёт экранную пропорцию `h/w`; golden не изменились (`committed_golden_grids_match_the_renderer`), регрессий нет — правка только в комментариях и строке README. Комментарии `options.rs`, `resize.rs` и строка README приведены к «ширина/высота»; поведение и golden не менялись — `cargo fmt`, `clippy`, `test` (200 тестов), `cov` (100% строк), `purity` зелёные |

Для `human-rejected` и `human-deferred` в «Чем подтверждено» пишется «человек, <дата>, <причина>».

## Журнал

| Цикл | Дата | Проход | Команды | Итог |
|---|---|---|---|---|
| 1 | 2026-11-27 | полный | `fmt`, `clippy`, `test` (200: pxcii-core 149, xtask 46, cli 4), `cov` (100% строк по каждому файлу; регионы 98.42% — допустимо п.4 DoD), `purity` — зелёные. Независимая реализация на Python (box, яркость гаммой 2.0, квантование, FS, Bayer 8×8, braille-биты по таблице Unicode, ANSI) воспроизвела все 6 golden побайтово. `xtask gen-fixtures` прогнан повторно — фикстуры и golden не изменились. `xterm` выбирает ровно ближайшую запись палитры на всех 4992 проверенных цветах; дрейф FS ≤ 2/255 на плоских полях. Аппроксимация sRGB гаммой 2.0 — отклонение от п.4 плана из-за запрета трансцендентных функций (раздел «Риски»): серый ряд точен (0 ошибки), на цветах отклонение ≤ 10/255 (RMS ≈ 3); точная кривая требует `powf` либо LUT 256 + обратный поиск — не менялось по KISS/YAGNI, вынесено человеку; решение человека 2026-11-27: гамма 2.0 остаётся, п.4 плана приведён в соответствие | OK |
| 1 | 2026-11-27 | верификация F-01 | `fmt`, `clippy`, `test` (200: pxcii-core 149, xtask 46+3, cli 1+1), `cov` (100% строк по каждому файлу; регионы 98.42%), `purity` — зелёные; демо `cargo run -p pxcii-core --example render -- fixtures/gradient.png` печатает ANSI; F-01 закрыт: статус `verified`, изменены только комментарии и строка README, поведение и golden не тронуты; новых замечаний нет | OK |
| 2 | 2026-11-27 | полный | `fmt`, `clippy`, `test` (200: pxcii-core 149, xtask 46+3, cli 1+1), `cov` (100% строк по каждому файлу; регионы 98.42%), `purity` — зелёные; демо `cargo run -p pxcii-core --example render -- fixtures/gradient.png` печатает ANSI. Проверены 5 отклонений от буквы плана, все соответствуют коду: `luma.rs:9-21` — гамма 2.0 (`encoded * encoded`, обратно `sqrt`), `powf`/`powi`/`sin`/`f64` в `pxcii-core/src` отсутствуют; `ramp.rs:7,34` — `BLOCKS = "░▒▓█"` как в плане, `glyphs()` разворачивает в `█▓▒░` (`blocks_are_listed_light_to_dark_but_indexed_dark_first`, golden `blocks-mono.txt`), все рампы индексируются тёмно→светло, как ASCII; `mod.rs:26,47-51,94` — Braille идёт отдельной веткой с `BRAILLE_THRESHOLD = 128`, `Dither` к ней не применяется (doc `render_bytes`, README:75); `grid.rs:24` + `mod.rs:72,99` — `Cell.bg` всегда `None`, `ansi.rs` и `text.rs` его не читают (`text.rs:63` — тест на игнор); Lanczos в `resize.rs` нет и выбора фильтра в `RenderOptions` нет, п.3 плана перечисляет box/Lanczos как альтернативы — мёртвого кода не появилось. Golden-гриды (6 шт.) совпадают с рендером (`committed_golden_grids_match_the_renderer`), README и CHANGELOG этапа заполнены; новых замечаний и регрессий нет | OK |
