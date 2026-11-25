# pxcii — детальный план разработки (инкрементальный, автономный)

Источник требований: `pxcii-tz.md`
Модель разработки: этап = готовый продукт + зелёные тесты + короткий отчёт для ревью.
Цель по ревью человеком: 10–30 минут на этап.

---

## 0. Ключевые решения (приняты на этапе планирования)

| Решение | Значение                                                                                         |
|---|--------------------------------------------------------------------------------------------------|
| Язык | Rust, edition 2021; тулчейн запинен в `rust-toolchain.toml` (1.100.0), `rust-version` в `Cargo.toml` = фактический минимум зависимостей |
| Workspace | `pxcii-core` (lib), `pxcii-py` (cdylib), `pxcii` (bin), `xtask` (служебный)                      |
| Бинарь | один `pxcii`, сабкоманды `render` (дефолт) / `desktop` / `serve`                                 |
| GUI | `eframe`/`egui` за фичей `gui` (в `default`), `default-features = false` + отключённая persistence |
| Web | `axum` + `tokio`, inline-страница, `POST /render`, лимит тела                                    |
| Python | `pyo3` (`abi3-py39`) отдельным крейтом, ядро не меняется                                         |
| Форматы | 9: PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, TGA, PNM — все pure Rust                                |
| C-зависимости | запрещены (в CI проверка `cargo tree` на отсутствие `-sys` крейтов с C)                          |
| Побочные эффекты | в библиотеке нет работы с ФС; запись только по явному пути из CLI/GUI/web                        |
| Тесты | `cargo test` + golden-файлы + `cargo llvm-cov`, цель 100% строк                                  |
| CI | GitHub Actions, `.github/workflows/ci.yml` — зафиксировано, GitLab CI не используется |
| Git-конфиг | изменения только через `git config --local`; глобальный и системный конфиг не трогаем |
| Установка ПО | агент ничего не ставит и не скачивает в систему без одобрения человека — только сообщает, что нужно |
| План и ТЗ | `pxcii-dev-plan.md` и `pxcii-tz.md` в корне репозитория |

### Границы крейтов

```
pxcii-core/   # decode → pipeline → Grid<Cell> → ansi|text|raster → encode
pxcii/        # bin: clap, три сабкоманды, файловый I/O (единственное место с путями)
pxcii-py/     # PyO3-обёртка над core (bytes/str на границе)
xtask/        # генерация фикстур, проверка зависимостей, релизные скрипты
```

Правило: `pxcii-core` не знает про `Path`, `std::fs`, `env`. Проверяется тестом
(грепом по исходникам в `xtask check-core-purity`) и ревью.

---

## 1. Команда агентов

Для пет-проекта **не нужна команда из многих ролей**. Нужны два агента:

| Агент | Роль | Модель | Режим |
|---|---|---|---|
| `pxcii-dev` | Основной исполнитель этапа: код, тесты, доки, CI, конфиги | `teamorouter-anthropic/claude-sonnet-5-5#medium` (на сложных этапах `#high`) | primary |
| `pxcii-review` | Приёмка этапа: прогон команд, поиск дыр в тестах, отчёт по чек-листу. Не правит код | `teamorouter-anthropic/deepseek-flash-1017#high` | subagent, read-only |

Почему так:

- Один исполнитель держит целостность архитектуры. Два-три «senior-разработчика» на 8–9 тысячах строк
  начнут расходиться в контрактах и тратить токены на синхронизацию — это дороже, чем выигрыш.
- Отдельный ревьюер оправдан: он «свежим контекстом» проверяет требование «100% покрытие» и
  отсутствие побочных эффектов, то есть делает ту работу, которую иначе делает человек 30 минут.
- Скилы не заводим: специализированных повторяемых workflow нет, всё покрывается AGENTS.md.
- Команды: две (`/stage`, `/verify`) — они экономят человеку ввод и фиксируют порядок действий.

### Модель по этапам (не перестраховываться)

| Этап | Модель | Обоснование |
|---|---|---|
| 1. Фундамент + decode | Sonnet 5.5 `medium` | конфиги + CI-матрица, ошибки компиляции дорогие |
| 2. Encode + roundtrip | Sonnet 5.5 `medium` | рутинные энкодеры, но много edge-case |
| 3. Render core | Sonnet 5.5 `high` | математика, дизеринг, golden-детерминизм |
| 4. CLI | DeepSeek Flash 1017 `high` | обвязка clap, шаблонная работа |
| 5. Rasterizer + image out | Sonnet 5.5 `medium` | шрифты/глифы, аккуратная геометрия |
| 6. Desktop | Sonnet 5.5 `medium` | API `eframe` меняется, нужна точность |
| 7. Web | DeepSeek Flash 1017 `high` | `axum`-бойлерплейт |
| 8. Python | Sonnet 5.5 `medium` | нюансы PyO3/abi3/maturin |
| 9. Release | DeepSeek Flash 1017 `medium` | YAML/Docker/доки |

Ревьюер на всех этапах: DeepSeek Flash 1017 `high`. Если отчёт ревьюера поверхностный
(меньше 3 конкретных замечаний или нет ссылок на файлы) — этап перепроверяется
Sonnet 5.5 `medium` вручную человеком.

Модель выбирает человек в TUI (`/models`) перед запуском этапа; в конфиге агента `pxcii-dev`
она не прописана, иначе перекрывала бы выбор. У `pxcii-review` модель закреплена — ревью всегда
идёт на одной модели.

### Протокол общения агентов (экономия токенов)

- Агент работает **файл за файлом**, не выводит содержимое файлов в чат без запроса.
- Отчёт об этапе — не более 40 строк: что сделано, команды проверки, что осталось,
  риски. Ссылки на файлы, а не цитаты.
- Промежуточные сообщения — одна-две строки («сгенерировал фикстуры, иду в энкодеры»).
- Ревьюер возвращает только список замечаний (severity + файл:строка), без пересказа кода.
- Если этап не влезает в один ответ — агент продолжает работу без пересказа сделанного.

---

## 2. Definition of Done (общий для всех этапов)

Этап считается закрытым, когда:

1. `cargo fmt --all --check` — чисто.
2. `cargo clippy --workspace --all-targets -- -D warnings` — чисто.
3. `cargo test --workspace --all-features` — зелёный.
4. `cargo llvm-cov --workspace --fail-under-lines 100` — 100% по строкам
   (исключения только через `#[coverage(off)]` с комментарием-обоснованием:
   `main()`, конструкторы окон egui, инициализация tokio-рантайма).
5. `xtask check-core-purity` — в `pxcii-core` нет `std::fs`, `Path`, `env::`.
6. Новая функциональность покрыта тестами **и** golden-файлами (если влияет на вывод).
7. `README.md` — обновлён раздел для появившейся возможности; `CHANGELOG.md` — запись.
8. Есть демонстрационная команда, которую человек запускает за 10 секунд.
9. Отчёт агента в чат — по формату выше.
10. Если для этапа нужен новый инструмент, шрифт или загрузка из сети — агент не ставит это сам,
    а пишет человеку: что нужно, зачем и какой командой; ждёт одобрения.

Коммиты: один этап = один squash-коммит с префиксом `stage-N: ...`.

---

## 3. Карта этапов

| # | Этап | Продукт этапа | Ревью |
|---|---|---|---|
| 1 | Фундамент + чтение форматов | библиотека декодирует 9 форматов, CI зелёный | 20 мин |
| 2 | Запись форматов + конвертация | библиотека конвертирует любой из 9 в любой из 9 | 20 мин |
| 3 | Ядро рендера | `&[u8]` → строка ANSI/текст с псевдографикой | 30 мин |
| 4 | CLI | `pxcii photo.jpg` печатает в консоль, сохраняет текст | 20 мин |
| 5 | Растеризация + вывод изображением | сохранение псевдографики в PNG/JPEG/… | 25 мин |
| 6 | Desktop | окно выбора файла, цвет/монохром, сохранение | 25 мин |
| 7 | Web | локальный сервер + страница | 20 мин |
| 8 | Python-пакет | `pip install` → рендер из кода | 25 мин |
| 9 | Релиз-обвязка | Docker slim, матрица артефактов, доки | 20 мин |

Итого ~3 ч 15 мин ревью на весь проект.

---

## Этап 0. Окружение и агенты (выполнен до этапа 1)

**Продукт:** воспроизводимое окружение и команда агентов, готовые к работе.

1. Rust: rustup + MSVC-тулчейн, `rustfmt`, `clippy`, `llvm-tools-preview`, `cargo-llvm-cov`.
   Версия запинена в `rust-toolchain.toml` (1.100.0). Системные предпосылки Windows:
   Visual Studio Build Tools с `VC.Tools.x86.x64` и Windows SDK — ставятся человеком вручную.
2. `git config --local core.autocrlf true`, `git config --local core.longpaths true`;
   глобальный и системный конфиг не меняются.
3. `.gitattributes`: `* text=auto eol=lf`, бинарные расширения помечены `binary` —
   результат не зависит от `core.autocrlf`.
4. `opencode.jsonc`, `.opencode/agents/pxcii-dev.md`, `.opencode/agents/pxcii-review.md`,
   `.opencode/commands/stage.md`, `.opencode/commands/verify.md`, `AGENTS.md`.

**Проверка:** `rustc -vV` даёт `host: x86_64-pc-windows-msvc`, `cargo llvm-cov --version` работает,
`cargo llvm-cov --summary-only` на тестовом крейте показывает покрытие.

---

## Этап 1. Фундамент + чтение форматов

**Продукт:** репозиторий собирается, CI зелёный, библиотека декодирует 9 форматов в `RgbaImage`.

**Состав работ**

1. `Cargo.toml` (workspace, `resolver = "2"`), `rust-toolchain.toml`, `.gitignore`
   (`/target`, `*.profraw`, `dist/`, `.venv`, `__pycache__`, `.pytest_cache`).
2. Крейты: `pxcii-core/src/lib.rs`, `pxcii/Cargo.toml` (bin-заглушка), `xtask`.
3. `pxcii-core/src/format.rs`:
   - `enum Format { Png, Jpeg, Webp, Gif, Bmp, Tiff, Ico, Tga, Pnm }`
   - `Format::from_ext(&str) -> Option<Format>` (`jpg|jpeg`, `tif|tiff`, `ppm|pgm|pbm|pnm`)
   - `Format::sniff(&[u8]) -> Option<Format>` по magic bytes (PNG 89 50 4E 47, JPEG FFD8FF,
     GIF "GIF8", BMP "BM", TIFF "II*\0"/"MM\0*", WebP "RIFF"+"WEBP", ICO 00 00 01 00,
     TGA — эвристика по 18-байтовому заголовку, PNM — "P1".."P6")
   - `Format::mime(&self)`, `Format::extension(&self)`, `Format::ALL`
4. `pxcii-core/src/decode.rs`: `decode(bytes: &[u8]) -> Result<RgbaImage, PxciiError>`
   - автоопределение: `sniff` → при неудаче `from_ext`-подсказка из вызывающего кода
   - сторонние крейты: `png`, `zune-jpeg`, `image-webp`, `gif`, `tiff`, `image` (bmp/ico/tga/pnm)
   - многокадровые GIF/TIFF/WebP → первый кадр
   - ориентация EXIF для JPEG (8 значений) — через `zune-jpeg`/ручной парсинг APP1
5. `pxcii-core/src/error.rs`: `enum PxciiError` на `thiserror`
   (`UnsupportedFormat`, `CorruptImage { format }`, `DimensionLimit { w, h }`,
   `Encode { format }`, `Io(std::io::Error)`), без паник на пользовательских данных.
6. Лимит входных данных: `MAX_PIXELS = 100_000_000` (проверяется после чтения заголовка).
7. `xtask`: подкоманды `gen-fixtures`, `check-core-purity`.
8. Фикстуры: `xtask gen-fixtures` генерирует программно (не бинарные блобы из сети)
   9 файлов 16×16 (`fixtures/…`): цветной градиент + альфа. Уже сгенерированные — коммитятся.
9. CI — **GitHub Actions** (`.github/workflows/ci.yml`; зафиксировано, GitLab CI не используется):
   fmt, clippy, test, coverage-артефакт, `check-core-purity`. Тулчейн берётся из
   `rust-toolchain.toml`, версию в workflow руками не дублировать.
10. `opencode.jsonc` + `.opencode/agents/*.md` + `.opencode/commands/*.md` + `AGENTS.md`
    (см. раздел «Конфигурация OpenCode» ниже) — созданы на этапе 0; на этом этапе проверить,
    что они на месте, и не пересоздавать.
11. `README.md` (заглушка с описанием и планом), `LICENSE-MIT`, `LICENSE-APACHE`.
12. `README.md` с первого этапа содержит раздел «Установка»: как поставить и как удалить Rust
    на Windows / Linux / macOS (rustup, проверка версии, `rustup self uninstall`) и полный
    порядок запуска проекта от `git clone` до первой команды. Раздел дополняется на каждом этапе.
    Плюс короткий раздел «Разработка»: команды `/stage N`, `/verify`, выбор модели, ссылка
    на раздел 7 плана.
13. `Cargo.toml` — `rust-version` выставляется по фактическому минимуму зависимостей
    (проверяется первым `cargo build`), а не 1.75 из ранней версии плана.

**Тесты**

- `format::tests`: расширения с точками/без/в верхнем регистре, все 9 magic bytes,
  мусорные байты → `None`, усечённые заголовки.
- `decode::tests`: по фикстуре на формат (`decode(fixture) == ожидаемый размер`),
  битый файл → `CorruptImage`, пустой срез → `UnsupportedFormat`, превышение `MAX_PIXELS` → `DimensionLimit`.
- `xtask check-core-purity`: тест на grep-правило.

**Демо:** `cargo run -p xtask -- gen-fixtures && cargo test -p pxcii-core`.

**Риски:** TGA не имеет magic bytes — сниффинг эвристический, обязателен fallback по расширению;
`zune-jpeg` не отдаёт EXIF — ориентация парсится вручную, покрыть тестом на 8 значений.

**Приёмка:** CI зелёный, все 9 форматов декодируются, `cargo tree` без C-`-sys`.

---

## Этап 2. Запись форматов + конвертация

**Продукт:** `convert(bytes, Format) -> Vec<u8>` — любой вход в любой из 9 выходов, детерминированно.

**Состав работ**

1. `pxcii-core/src/encode.rs`:
   - `encode(img: &RgbaImage, format: Format, opts: &EncodeOptions) -> Result<Vec<u8>, PxciiError>`
   - `EncodeOptions { quality: u8 (JPEG, default 100), lossless: bool }`
   - PNG (`png`), JPEG (`jpeg-encoder`, 4:4:4, quality 100 по умолчанию),
     WebP (`image-webp`, только lossless), GIF (`gif` + `color_quant`,
     Floyd–Steinberg до 256 цветов), BMP/TGA/ICO/PNM (`image`), TIFF (`tiff`, deflate)
2. Ограничения, реализованные явно и с предупреждением `Vec<Warning>`, а не молча:
   - ICO > 256×256 → ресайз (box) до 256 по длинной стороне, `Warning::IcoDownscaled`
   - GIF → квантование в 256 цветов, `Warning::GifQuantized`
   - JPEG → `Warning::JpegArtifacts` (подсказка использовать PNG)
   - прозрачность в JPEG/BMP/TIFF/PNM → композит на чёрном, `Warning::AlphaDropped`
3. `pxcii-core/src/convert.rs`: `convert(bytes, format, opts) -> Result<Converted>`.
4. Ресайз: `pxcii-core/src/resize.rs` — box-фильтр (для ICO) на целочисленной арифметике.

**Тесты**

- Roundtrip-матрица 9×9 на фикстурах: `decode(encode(decode(f)))` даёт те же размеры;
  для lossless-форматов (PNG, BMP, TGA, TIFF, PNM, WebP-lossless) — **побайтовое** сравнение
  хешей с golden-файлами в `fixtures/golden/`.
- Детерминизм: два прогона одного энкода → идентичные байты (для всех 9).
- Ограничения: ICO 300×300 → 256×256 + warning; GIF с 5000 цветов → ≤256; JPEG с альфой → warning.
- Негативные: `encode` в формат при неподдерживаемом размере → явная ошибка/предупреждение.

**Демо:** `cargo run -p xtask -- convert fixtures/gradient.png --to webp`.

**Приёмка:** матрица 9×9 проходит, golden-байты стабильны, предупреждения видны в API.

---

## Этап 3. Ядро рендера (псевдографика)

**Продукт:** `render(bytes, &RenderOptions) -> Grid` + `to_ansi(&Grid) -> String` + `to_text(&Grid) -> String`.
Сердце продукта, самый важный этап по качеству.

**Состав работ**

1. `pxcii-core/src/render/options.rs`:
   ```rust
   pub struct RenderOptions {
       pub cols: u32,             // по умолчанию 80
       pub color: ColorMode,      // TrueColor | Ansi256 | Mono
       pub ramp: Ramp,            // Ascii | Blocks | Braille | Custom(String)
       pub dither: Dither,        // None | FloydSteinberg | Ordered
       pub cell_aspect: f32,      // по умолчанию 0.5 (высота/ширина знакоместа)
   }
   ```
2. `render/grid.rs`: `struct Cell { ch: char, fg: Rgb, bg: Option<Rgb> }`, `Grid { cols, rows, cells }`.
3. `render/resize.rs`: приведение к сетке `cols × rows`, где
   `rows = round(cols * (h/w) * cell_aspect)`, фильтр box/Lanczos на `f32`.
4. `render/luma.rs`: sRGB → linear (`f32`), яркость `0.2126R + 0.7152G + 0.0722B`, обратно в sRGB.
5. `render/ramp.rs`: `Ascii "@%#*+=-:. "`, `Blocks "░▒▓█"`, `Braille` (2×4 субпикселя).
   Для Braille — отдельная ветка: одна ячейка = 8 отсчётов, битовая маска → `char`.
6. `render/dither.rs`: Floyd–Steinberg (только целочисленный, ошибка в 1/16 долях) и
   Bayer 8×8 ordered (целочисленная матрица). Никаких `sin`/`powi` — иначе разъезд по платформам.
7. `render/ansi.rs`: truecolor `\x1b[38;2;r;g;bm`, `Ansi256` через куб 6×6×6 + градации серого,
   `Mono` без escape-кодов; сброс `\x1b[0m` в конце строки.
8. `render/text.rs`: plain-текст, `bg` игнорируется.
9. `render/mod.rs`: `render_bytes(&[u8], &RenderOptions) -> Result<Grid, PxciiError>`.

**Тесты**

- Golden-гриды: для 6 конфигураций (ascii/color, block/mono, braille/mono, dither×2, aspect 0.5/1.0)
  в `fixtures/golden/grids/*.txt` — сравнение построчно.
- Свойства (property-тесты без `proptest`, простым перебором):
  - пустое изображение 1×1 не паникует;
  - `rows >= 1` при любых `cols ∈ 1..400`;
  - mono-вывод не содержит `\x1b[`;
  - каждая строка `to_text` имеет ровно `cols` символов (кроме широких — запрещены ramp'ом).
- Дизеринг: градиент 256×1 → число уникальных символов монотонно растёт при `dither != None`.
- Braille: известный вход 2×4 → известный кодпоинт (табличный тест на 8 бит).
- `cell_aspect`: `aspect = 2.0` даёт `rows` вдвое больше, чем `1.0`.

**Демо:** пример `examples/render.rs` — читает путь из argv, печатает ANSI.

**Риски:** детерминизм float-ресайза между платформами. Меры: только `f32` без трансцендентных
функций, golden-тесты сравнивают **строки сетки**, а не байты изображения.

**Приёмка:** визуально узнаваемо на фото 500×500, golden стабильны, 100% покрытие `render/*`.

---

## Этап 4. CLI

**Продукт:** полноценная консольная утилита по ТЗ (п. 4–5), без вывода в изображение
(это следующий этап).

```bash
pxcii photo.jpg                      # ANSI в консоль
pxcii photo.jpg --mono --cols 100    # монохром, ширина 100
pxcii photo.jpg -o art.txt           # сохранить текст
pxcii photo.jpg --ramp blocks --dither fs --aspect 0.5
```

**Состав работ**

1. `pxcii/src/main.rs`: `clap` derive, сабкоманды `Render` (дефолт при отсутствии),
   `Desktop` (заглушка `bail!`, настоящая — этап 6), `Serve` (заглушка, этап 7).
2. `pxcii/src/cli/render_cmd.rs`:
   - аргументы: `INPUT`, `-o/--output <PATH>`, `--cols <u32>`, `--color truecolor|256|mono`,
     `--mono` (алиас `--color mono`), `--ramp ascii|blocks|braille`, `--dither none|fs|ordered`,
     `--aspect <f32>`, `--format <ext>` (для вывода изображением, этап 5), `--force`
   - разрешение конфликтов: `--mono` + `--color truecolor` → ошибка clap `conflicts_with`
3. `pxcii/src/cli/output.rs`: определение «текст или изображение» — по расширению `-o`
   (`Format::from_ext`) и явному `--format`; `-` или отсутствие `-o` → stdout.
4. Коды выхода: `0` успех, `1` ошибка ввода/файла, `2` ошибка аргументов (clap), `3` неподдерживаемый формат.
5. `pxcii/src/cli/report.rs`: предупреждения из `encode`/`decode` → stderr с префиксом `hint:`.
6. Windows: включение VT-режима для ANSI (`ansi_term`-подобный `SetConsoleMode` через `windows-sys`
   только под `cfg(windows)`; в остальных ОС — no-op).
7. `xtask check-core-purity` расширяется правилом: `pxcii-core` не знает про `clap`.

**Тесты**

- `assert_cmd` + `predicates`: `pxcii fixtures/gradient.png` → exit 0, stdout содержит ANSI;
  `--mono` → без escape-кодов; `-o out.txt` создаёт файл ровно с `cols` символами в строке;
  несуществующий файл → exit 1 + сообщение; `--ramp nope` → exit 2.
- Пустой путь, путь к каталогу, нет прав на запись → exit 1, без паники.
- Snapshot-тест stderr для `--format jpeg` (предупреждение об артефактах).

**Демо:** `cargo run -p pxcii -- fixtures/gradient.png --cols 40`.

**Приёмка:** все 9 форматов на входе читаются, вывод в консоль и в текст работает,
Windows-сборка показывает цвет в `cmd.exe`.

---

## Этап 5. Растеризация сетки и вывод изображением

**Продукт:** `-o art.png` (и 8 других форматов) даёт картинку с псевдографикой.

**Состав работ**

1. `pxcii-core/src/raster/mod.rs`: `rasterize(grid: &Grid, spec: &RasterSpec) -> RgbaImage`
   ```rust
   pub struct RasterSpec { pub cell: (u32, u32), pub font: Option<&FontSpec>, pub bg: Rgb }
   ```
2. Две ветки:
   - **геометрическая** (ramp `Blocks`, `Braille`): ячейка рисуется попиксельно
     (`█` — заливка, `░▒▓` — по маске, braille — 8 субпикселей). Шрифт не нужен,
     результат детерминирован побитово.
   - **глифовая** (ramp `Ascii`, `Custom`): `ab_glyph` + встроенный шрифт
     `assets/fonts/JetBrainsMono-Regular.ttf` (OFL, `include_bytes!`), рендер без сглаживания
     (`threshold` по покрытию), иначе границы «мылятся».
3. `raster/font.rs`: `FontSpec { data: &'static [u8], px: u32 }`, кэш растровых глифов
   `HashMap<char, Bitmap>` внутри вызова (без глобального состояния).
4. `pxcii/src/cli/render_cmd.rs`: при выводе в изображение — `rasterize` + `encode`,
   выбор `cell` из `--cell WxH` (по умолчанию `8x16` для ascii, `1x2`-кратное для blocks).
5. `Warning::RasterNeedsFont` при кастомном ramp с не-ASCII символом вне покрытия шрифта →
   замена на `?` + предупреждение.

**Тесты**

- Golden PNG: `rasterize` blocks-ramp 10×4 → хеш совпадает с `fixtures/golden/raster-blocks.png`
  (сравнение декодированных пикселей, не байтов файла — энкодер может менять метаданные).
- Геометрия: ячейка `█` в (0,0) при `cell=(4,4)` → все 16 пикселей цвета `fg`.
- `░` даёт ровно 1/4 закрашенных пикселей, `▒` — 1/2, `▓` — 3/4 (допуск ±1 пиксель на границе).
- Шрифт: все символы ASCII-ramp отрисованы (непустая маска), `?`-fallback срабатывает.
- Roundtrip: PNG → rasterize → PNG → ожидаемые размеры `cols*cell.w × rows*cell.h`.

**Демо:** `cargo run -p pxcii -- fixtures/photo.png -o art.png --cols 120 && start art.png`.

**Риски:** лицензия шрифта — положить `OFL.txt` рядом и указать в `NOTICE`; размер бинаря +400 КБ.

**Приёмка:** PNG-выход открывается и читается глазами, golden стабильны, `--format jpeg` работает.

---

## Этап 6. Desktop-режим

**Продукт:** `pxcii desktop` — окно с выбором файла, превью, переключателем цвет/монохром и сохранением.

**Состав работ**

1. Фича `gui` в `pxcii/Cargo.toml`: `eframe` c `default-features = false`
   (`glow`-бэкенд, без `persistence`, без `accesskit`), `rfd` для диалогов.
2. `pxcii/src/desktop/mod.rs` — тонкий слой: `eframe::run_native`, без логики.
3. `pxcii/src/desktop/state.rs` — **вся логика, тестируемая без окна**:
   ```rust
   pub struct AppState {
       input: Option<InputImage>,   // bytes в памяти
       options: RenderOptions,
       preview: Option<String>,     // ANSI не нужен — рисуем Grid как текстуру
       status: Status,
   }
   impl AppState {
       pub fn open_bytes(&mut self, data: Vec<u8>) -> Result<(), PxciiError>;
       pub fn set_mono(&mut self, mono: bool);
       pub fn rerender(&mut self) -> Result<(), PxciiError>;
       pub fn save_as(&self, path: &Path, target: SaveTarget) -> Result<(), PxciiError>;
   }
   ```
4. Drag-and-drop через `egui::Context::input(|i| i.raw.dropped_files)` → тот же `open_bytes`.
5. Превью: `Grid` → текстура `egui::TextureHandle`, пересоздаётся при смене опций (кэш в памяти окна).
6. **Запрет побочных эффектов**, реализованный механически:
   - `eframe` без фичи `persistence` (проверить `cargo tree -e features | grep persistence` в CI);
   - нет `dirs`/`directories`/`tempfile` в зависимостях (проверяется `xtask check-core-purity`);
   - тест `desktop` под `#[cfg(test)]` обходит `AppState` и проверяет, что после серии
     операций список файлов в рабочем каталоге не изменился.
7. Ошибки — в статус-строке окна, не в stderr и не в модальных диалогах.

**Тесты**

- `AppState`: загрузка 9 форматов, `set_mono` пересчитывает превью, `save_as` текст и изображение,
  битые байты → `Status::Error` без паники, открытие нового файла заменяет предыдущий (утечки нет).
- `save_as` в несуществующий каталог → ошибка, файл не создаётся.
- Отсутствие побочных эффектов: снимок `read_dir(".")` до и после — равен.
- `#[coverage(off)]` только на `run_native` и `impl eframe::App for ...` (обосновано комментарием).

**Демо:** `cargo run -p pxcii --features gui -- desktop`.

**Риски:** `eframe` подтягивает `winit`/`wgpu`; на Linux-сервере режим недоступен — это ожидаемо
(см. ТЗ п. 2: slim-сборка для Docker). В CI GUI-тесты идут только на ubuntu/windows/macos раннерах
без `xvfb` (тестируется `AppState`, не окно).

**Приёмка:** окно открывается, файл выбирается, превью меняется, сохранение работает,
после запуска в домашнем каталоге не появилось новых файлов.

---

## Этап 7. Web-режим

**Продукт:** `pxcii serve --port 8080` → страница с выбором файла, цвет/монохром, сохранение.

**Состав работ**

1. `pxcii/src/web/mod.rs`: `axum` 0.7 + `tokio`, роуты:
   - `GET /` → встроенный HTML (`include_str!`, один файл, без CDN, без внешних ресурсов)
   - `POST /render` → тело `multipart/form-data` (`file`, `mono`, `cols`, `ramp`, `dither`);
     ответ `text/plain; charset=utf-8` (ANSI) или `image/png` при `Accept: image/png`/`format=png`
   - `GET /healthz` → `ok`
2. `DefaultBodyLimit::max(32 * 1024 * 1024)` — лимит тела, превышение → `413`.
3. Всё в памяти: `Bytes` → `decode` → `render` → ответ. **Ноль** `tempfile`, ноль записи на диск.
4. Ошибки: `PxciiError` → JSON `{"error": "...", "kind": "..."}` + корректный HTTP-код
   (415 неподдерживаемый формат, 422 битое изображение, 413 слишком большой файл).
5. `--host`/`--port`/`--open` (открыть браузер), по умолчанию `127.0.0.1:8080` — не `0.0.0.0`.
6. Логи: только в stdout, уровень `warn`+; никаких файлов логов.
7. Страница: `<input type="file">`, drag-and-drop, чекбокс «монохром», `<pre>` с результатом,
   кнопка «Сохранить» (`Blob` + `a.download`), кнопка «Сохранить PNG» (Blob из ответа сервера).

**Тесты**

- `tower::ServiceExt::oneshot` без реального сетевого порта:
  - `POST /render` с фикстурой PNG → 200, тело непустое, содержит ожидаемые строки;
  - `mono=true` → ответ без `\x1b[`;
  - `format=png` → `content-type: image/png`, байты декодируются в PNG;
  - битый файл → 422; `.heic`-подобный мусор → 415; тело > лимита → 413;
  - `GET /healthz` → 200.
- Тест «нет побочных эффектов»: снимок `read_dir` рабочего каталога и системного temp до/после
  серии запросов — равен.
- HTML-страница: тест на отсутствие внешних `http(s)://` ссылок (`include_str!` + `contains`).

**Демо:** `cargo run -p pxcii -- serve --port 8080` → открыть `http://127.0.0.1:8080`.

**Приёмка:** загрузка через браузер работает, лимиты срабатывают, на диске ничего не появилось.

---

## Этап 8. Python-пакет

**Продукт:** `pip install pxcii` → `pxcii.render_ansi(...)`, `pxcii.render_png(...)`,
`pxcii.convert(...)` — только open/save в коде.

**Состав работ**

1. Крейт `pxcii-py`: `crate-type = ["cdylib", "rlib"]`, `pyo3 = { version, features =
   ["extension-module", "abi3-py39"] }` за фичей, чтобы `cargo test` не падал на линковке.
2. Публичный API модуля:
   ```python
   pxcii.render_text(data: bytes | str(path), *, cols=80, color="truecolor",
                     ramp="ascii", dither="none", aspect=0.5) -> str
   pxcii.render_ansi(...) -> str
   pxcii.render_image(data, *, fmt="png", cols=80, ...) -> bytes
   pxcii.convert(data, fmt) -> bytes
   pxcii.save_ansi(data, path, ...) -> None
   pxcii.save_image(data, path, *, fmt=None, ...) -> None   # fmt из расширения
   ```
3. Никаких `desktop`/`serve`/CLI в пакете — проверяется тестом, что модуль не экспортирует
   эти имена (`hasattr(pxcii, "serve") is False`).
4. GIL: `py.allow_threads(|| core::...)` на всех вызовах рендера/энкода.
5. Ошибки: `PxciiError(ValueError)` и подклассы `UnsupportedFormat`, `CorruptImage`, `DimensionLimit`.
6. `pxcii.pyi` (стабы) + `py.typed`, `python/pyproject.toml` (maturin, `abi3`), `README` секция.
7. Тесты: `pytest` в `python/tests/` — фикстуры те же, что в Rust; плюс `cargo test -p pxcii-py --no-default-features`.

**Тесты (Python)**

- 9 форматов: `render_image` → `convert` → байты декодируются Pillow (только в тестах).
- `render_text` даёт `cols` символов в строке; `color="mono"` без escape.
- `data` как `bytes` и как `str`-путь дают одинаковый результат; путь к отсутствующему файлу
  → `PxciiError`.
- Проверка отсутствия `serve`/`desktop`/`cli` в модуле.
- Проверка, что вызов не создаёт файлов (pytest `tmp_path` + снимок cwd).
- Совпадение результата Rust-эталона: один golden-ANSI, сгенерированный `xtask`, сравнивается
  в Rust-тесте и в pytest.

**Демо:** `pip install target/wheels/*.whl` → 3 строки Python.

**Риски:** `extension-module` ломает линковку тестов крейта — фича выключена по умолчанию,
включается в `[features] extension-module` при `maturin build`.

**Приёмка:** колесо ставится на чистом venv, импорт работает, тесты зелёные.

---

## Этап 9. Релиз-обвязка

**Продукт:** можно скачать бинарь и запустить Docker.

**Состав работ**

1. CI-матрица: `ubuntu-latest`, `windows-latest`, `macos-14` (arm64); цели
   `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`,
   `aarch64-apple-darwin`. Артефакты: `pxcii-{os}-{arch}` + `sha256`.
2. Job `slim`: `cargo build --release --no-default-features` → проверка размера (< 8 МБ) и
   `otool/objdump`-подобная проверка отсутствия GL/Vulkan-импортов (на Linux — `ldd` без `libGL`).
3. `Dockerfile`: multi-stage, `rust:1.100-slim` → `debian:bookworm-slim`, non-root,
   `EXPOSE 8080`, `ENTRYPOINT ["pxcii", "serve", "--host", "0.0.0.0"]`. `.dockerignore`.
4. `docker-compose.yml` для локального запуска — опционально, одна услуга.
5. GitHub Actions release-workflow (tag `v*`) — сборка + wheels через `maturin` +
   `PyPI`-публикация в `--dry-run` режиме по умолчанию.
6. `README.md`: полная установка от `git clone` (git-конфиг только `--local`, rustup,
   версия из `rust-toolchain.toml`, `rustup component add`, `cargo llvm-cov`), установка
   и удаление Rust для Windows / Linux / macOS (`rustup self uninstall`), установка из
   бинарников / Docker / pip, все три режима, 9 форматов, ограничения, примеры, раздел
   «Приватность: ничего не сохраняется».
7. `CHANGELOG.md`, `NOTICE` (шрифт OFL), `benches/` (`criterion`) — один бенч `render 1920×1080`,
   порог-регрессия в CI не блокирующий, только отчёт.
8. Coverage-бейдж/отчёт как артефакт CI.

**Тесты/проверки**

- Smoke-тест Docker-образа в CI: `docker run` + `curl /healthz` + `curl -F file=@fixture /render`.
- Проверка, что slim-бинарь не содержит `eframe` в `cargo tree`.
- Проверка воспроизводимости: два прогона CI дают одинаковый sha256 бинаря (без `--remap-path-prefix`
  возможно расхождение — тогда фиксируем `--locked` и `SOURCE_DATE_EPOCH`, а побайтовую
  воспроизводимость не обещаем).

**Демо:** `docker run -p 8080:8080 ghcr.io/…/pxcii` → страница работает.

**Приёмка:** релизный тег собирает 4 бинаря + колесо, Docker-образ проходит smoke, README описывает всё ТЗ.

---

## 4. Конфигурация OpenCode (создаётся на этапе 0)

Всё, кроме корневого `opencode.jsonc`, — в `.opencode/`. Глобальный конфиг не трогаем.

```
opencode.jsonc
AGENTS.md
pxcii-dev-plan.md
pxcii-tz.md
.opencode/
├── agents/
│   ├── pxcii-dev.md
│   └── pxcii-review.md
└── commands/
    ├── stage.md
    └── verify.md
```

`opencode.jsonc`:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "default_agent": "pxcii-dev",
  "agents": {
    "pxcii-dev": { "mode": "primary" },
    "pxcii-review": { "mode": "subagent" }
  }
}
```

`.opencode/agents/pxcii-dev.md`:

```md
---
description: Исполнитель этапов pxcii — пишет код, тесты, доки и CI по плану
mode: primary
permissions:
  - action: edit
    resource: "*"
    effect: allow
  - action: shell
    resource: "*"
    effect: allow
  - action: subagent
    resource: "pxcii-review"
    effect: allow
---
```

Модель у `pxcii-dev` намеренно **не** прописана: поле `model` в агенте перекрывает модель,
выбранную человеком в TUI, а по плану модель меняется от этапа к этапу. Выбор — `/models`.

Тело агента (коротко, 10–15 строк): прочитать `AGENTS.md` и план этапов; выполнять только
запрошенный этап; Definition of Done; отчёт ≤ 40 строк; не пересказывать код; при нехватке
контекста — читать файлы, а не спрашивать.

`.opencode/agents/pxcii-review.md`:

```md
---
description: Приёмка этапа — проверяет DoD, тесты и побочные эффекты, код не правит
mode: subagent
model: teamorouter-anthropic/deepseek-flash-1017#high
permissions:
  - action: edit
    resource: "*"
    effect: deny
  - action: shell
    resource: "*"
    effect: deny
  - action: shell
    resource: "cargo *"
    effect: allow
  - action: shell
    resource: "git diff*"
    effect: allow
  - action: shell
    resource: "git status*"
    effect: allow
  - action: shell
    resource: "git log*"
    effect: allow
  - action: shell
    resource: "git show*"
    effect: allow
---
```

Порядок правил важен: выигрывает **последнее совпавшее**, поэтому общий `deny shell *`
стоит первым, а разрешения — после него. Обратный порядок обнулил бы все разрешения.

Тело: прогнать `fmt`/`clippy`/`test`/`coverage`/`check-core-purity`, проверить покрытие по файлам,
вернуть список замечаний `severity — файл:строка — суть` (максимум 10), без пересказа.

Команды:

- `.opencode/commands/stage.md` — `agent: pxcii-dev`, шаблон: «Выполни этап $1 плана,
  следуй Definition of Done, отчёт ≤ 40 строк».
- `.opencode/commands/verify.md` — `agent: pxcii-review`, `subagent: true`, шаблон:
  «Проверь текущий этап по чек-листу DoD. Только замечания».

Модель для конкретного этапа человек переключает в TUI по таблице раздела 1 либо
передаёт `/stage 3` при уже выбранной модели.

---

## 5. AGENTS.md (создаётся на этапе 0)

Короткий (1 экран), потому что дублировать план вредно:

1. Что за проект, стек, структура крейтов (5 строк).
2. Жёсткие ограничения: pure Rust, никаких C-`-sys`, `pxcii-core` без ФС, GUI без persistence,
   web без temp-файлов, один бинарь + фича `gui`.
3. Команды проверки (ровно те, что в DoD).
4. Правила тестов: golden-файлы только через `xtask gen-fixtures`, никаких сетевых фикстур,
   никаких `sleep` в тестах.
5. Стиль: rustfmt default, clippy `-D warnings`, docstring/`//!` по-русски или по-английски — выбрать
   одно (рекомендация: английский в коде, русский в документации и отчётах).
6. Ссылка на план: `pxcii-dev-plan.md` — единственный источник этапов.
7. Git-конфиг: изменения только через `git config --local`; глобальный и системный не трогать.
8. Запрет на самостоятельную установку: агент ничего не ставит и не скачивает в систему без
   одобрения человека — сообщает, что нужно и зачем. Исключение — DoD-команды, которые тянут
   крейты из crates.io в `~/.cargo`.
9. Переводы строк: LF, отвечает `.gitattributes`; файлы с CRLF не коммитить.

Те же правила 1, 8 и 9 дублируются в теле агента `pxcii-dev` — как напоминание в каждом сеансе.

---

## 6. Риски и заранее принятые решения

| Риск | Решение |
|---|---|
| Float-разъезд golden между ОС | целочисленный дизеринг, сравнение сетки строк, а не байтов |
| `eframe` тянет persistence | фича выключена + CI-проверка `cargo tree` |
| Шрифт в бинаре +400 КБ | приемлемо; альтернатива — только блочный ramp |
| JPEG-выход мылит глифы | warning + README-рекомендация PNG |
| TGA не сниффится | fallback по расширению, тест на эвристику |
| Rust-тесты PyO3 | `extension-module` только при сборке колеса |
| 100% покрытие недостижимо в GUI | логика в `AppState`, окно под `#[coverage(off)]` |
| Агент «раздувает» этап | DoD + отчёт ≤ 40 строк + ревьюер на 10 замечаний |
| MSRV 1.75 из ранней версии плана | нереалистичен для зависимостей 2026: тулчейн запинен в `rust-toolchain.toml`, `rust-version` = фактический минимум |
| `core.autocrlf=true` приходит из системного git-конфига | `.gitattributes` с `eol=lf`; git-настройки только `--local` |
| Агент сам ставит ПО и напихивает систему | правило в `AGENTS.md` и в теле агента: только сообщать человеку |

---

## 7. Как человеку работать с агентами

### Перед началом

- Окружение этапа 0 готово: rustup + MSVC Build Tools, `cargo-llvm-cov`, git-идентичность,
  `.gitattributes`, локальные git-настройки.
- OpenCode открыт **в корне репозитория** — иначе не подхватятся `opencode.jsonc`,
  `.opencode/agents/` и `.opencode/commands/`.
- Новая сессия стартует под `pxcii-dev` (`default_agent` в `opencode.jsonc`).
- Запрос агента на установку/скачивание человек подтверждает или отклоняет вручную.

### Выбор модели

Модель выбирается в TUI командой `/models` и действует на текущую сессию; конфиг при этом
не меняется. По таблице раздела 1:

| Этапы | Модель |
|---|---|
| 1, 2, 5, 6, 8 | `teamorouter-anthropic/claude-sonnet-5-5#medium` |
| 3 | `teamorouter-anthropic/claude-sonnet-5-5#high` |
| 4, 7 | `teamorouter-anthropic/deepseek-flash-1017#high` |
| 9 | `teamorouter-anthropic/deepseek-flash-1017#medium` |

У `pxcii-dev` модель не прописана — иначе значение из конфига перекрывало бы выбор в TUI.
У `pxcii-review` модель закреплена (`deepseek-flash-1017#high`), ревью всегда одинаковое.

### Команды

| Действие | Что ввести |
|---|---|
| начать этап | `/stage 1` … `/stage 9` |
| приёмка этапа | `/verify` |
| выбрать модель | `/models` |
| перечитать изменённые конфиги и агентов | `/reload` |

`/stage N` переключает сессию на `pxcii-dev` и отправляет промпт «Выполни этап N из
`pxcii-dev-plan.md` по Definition of Done». `/verify` запускает `pxcii-review` в фоновом
дочернем сеансе: основной чат остаётся доступным, по завершении приходит отчёт.

Те же действия текстом, если не хочется команд:

- «Выполни этап 3 из `pxcii-dev-plan.md` по Definition of Done. Отчёт не больше 40 строк.»
- «Проверь текущий этап по чек-листу DoD. Только замечания.»

### Цикл одного этапа

1. `/models` — выставить модель этапа.
2. `/stage N` — дождаться отчёта (≤ 40 строк) и списка прогнанных команд.
3. Прочитать дифф: `git status`, `git diff`, при желании самому прогнать DoD-команды (15–20 мин).
4. `/verify` — получить до 10 замечаний `severity — файл:строка — суть` или `OK`.
5. Замечания уровня blocker — отправить агенту текстом: «Исправь замечания 1, 3, 5».
   Затем снова `/verify`.
6. Запустить демо-команду этапа руками; на этапах 6 и 7 — обязательно окно и браузер.
7. Закоммитить: `git add -A && git commit -m "stage-N: ..."` — один этап, один коммит.

### Полезные фразы

| Задача | Промпт |
|---|---|
| не дать агенту забежать вперёд | «Только этап N, дальше не идти» |
| сначала план правок | «Покажи план изменений перед правкой файлов» |
| поймать пропуски в DoD | «Что из Definition of Done осталось невыполненным и почему?» |
| добить покрытие | «Добавь тесты на непокрытые строки, перечисли файлы» |
| вернуть этап назад | «Стоп. Вернись к этапу N, изменения после него отмени» |

### Чего агент не делает без вас

- не устанавливает ПО и не скачивает шрифты/архивы — сначала спросит;
- не меняет глобальный и системный git-конфиг (только `--local`);
- не коммитит и не пушит;
- не правит план, ТЗ и `AGENTS.md` без указания.

### Если что-то пошло не так

- агент делает не тот этап — «Стоп. Вернись к этапу N, изменения после него отмени»;
- падает `clippy`/тесты на чужом коде — прислать текст ошибки: «Починить без изменения поведения»;
- покрытие ниже 100% — «Добавь тесты на непокрытые строки, перечисли файлы»;
- ревьюер ругается на стиль — правит `pxcii-dev`, ревьюер код не трогает.



