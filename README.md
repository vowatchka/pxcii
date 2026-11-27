# pxcii

Конвертер изображений в псевдографику: один бинарь, три режима (CLI, desktop, web), плюс
Python-обёртка. Ядро — чистый Rust без C-зависимостей, весь пайплайн работает в памяти.

## Статус

Идёт разработка по этапам, источник истины — [`pxcii-dev-plan.md`](pxcii-dev-plan.md),
требования — [`pxcii-tz.md`](pxcii-tz.md).

| Этап | Состояние |
|---|---|
| 1. Фундамент + чтение форматов | **готов**: библиотека декодирует 9 форматов |
| 2. Запись форматов + конвертация | **готов**: `convert` любой из 9 форматов в любой из 9 |
| 3. Ядро рендера | **готов**: `render_bytes` → `Grid` → ANSI или текст |
| 4. CLI | не начат |
| 5. Растеризация | не начат |
| 6. Desktop | не начат |
| 7. Web | не начат |
| 8. Python-пакет | не начат |
| 9. Релиз-обвязка | не начат |

На этом этапе `pxcii` — заглушка, которая печатает баннер: CLI появляется на этапе 4.
Полезная функциональность пока живёт в библиотеке `pxcii-core` (чтение и запись форматов,
рендер в псевдографику).

## Поддерживаемые форматы

Вход (этап 1) и выход (этап 2): **PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, TGA, PNM**.

Ограничения, принятые осознанно:

- многокадровые GIF/TIFF/WebP читаются по первому кадру;
- вход ограничен 100 000 000 пикселей, лимит проверяется по заголовку файла;
- HEIC, AVIF, SVG и QOI не поддерживаются — требуют C-библиотек или вне задачи.

Ограничения записи (этап 2) не молчаливые: `encode` возвращает `Vec<Warning>`.

| Формат | Что происходит | Предупреждение |
|---|---|---|
| ICO | сторона больше 256 px: уменьшение box-фильтром до 256 по длинной стороне | `IcoDownscaled` |
| GIF | палитра 256 цветов (Floyd–Steinberg); прозрачность только полная (альфа < 128 — прозрачный пиксель) | `GifQuantized` |
| JPEG | с потерями, 4:4:4, качество 100 по умолчанию; для точной копии берите PNG | `JpegArtifacts` |
| JPEG, BMP, TIFF, PNM | нет альфа-канала: изображение накладывается на чёрный | `AlphaDropped` |
| WebP | только lossless; `lossless: false` — ошибка `Encode` | — |

Выход PNM — бинарный P6. TIFF пишется со сжатием deflate. Результат детерминирован: одинаковый
вход и опции дают одинаковые байты (шесть lossless-форматов закреплены golden-файлами в
`fixtures/golden/`).

TGA не имеет сигнатуры, поэтому определяется эвристикой по 18-байтовому заголовку с откатом
на расширение файла. TIFF декодируется в моделях gray, RGB и RGBA; палитровые файлы и
gray+alpha возвращают `CorruptImage` — это осознанное ограничение первого этапа.

Переводы строк задаёт `.gitattributes` (`eol=lf`), поэтому настраивать `core.autocrlf` не нужно.
Только на Windows с длинными путями может понадобиться локальная настройка репозитория:
`git config --local core.longpaths true` (глобальный конфиг не требуется).

## Рендер в псевдографику

`pxcii_core::render_bytes(&[u8], &RenderOptions) -> Grid` декодирует изображение и раскладывает
его по сетке символов. `Grid` хранит ячейки построчно, `to_ansi` печатает их с цветом,
`to_text` — без него.

| Параметр | Значения | По умолчанию |
|---|---|---|
| `cols` | ширина сетки в символах | 80 |
| `color` | `TrueColor`, `Ansi256`, `Mono` | `TrueColor` |
| `ramp` | `Ascii` (`@%#*+=-:. `), `Blocks` (`░▒▓█`), `Braille`, `Custom(String)` | `Ascii` |
| `dither` | `None`, `FloydSteinberg`, `Ordered` (Bayer 8×8) | `None` |
| `cell_aspect` | ширина знакоместа / высота | 0.5 |

Число строк — `round(cols * (h/w) * cell_aspect)`, минимум одна. Каждая ячейка — среднее
цвета исходной площади (box-фильтр на `f32`); яркость считается в линейном свете и возвращается
в sRGB. Braille — отдельная ветка: ячейка это 2×4 точки, порог яркости 128, дизеринг к ней
не применяется.

Детерминизм: дизеринг только целочисленный, гамма-коррекция — умножение вместо `powf`, поэтому
golden-гриды совпадают побайтово на всех платформах. Шесть конфигураций закреплены в
`fixtures/golden/grids/*.txt` и генерируются только командой `xtask gen-fixtures`.

## Установка Rust

Проект собран под Rust 1.100.0: версия запинена в [`rust-toolchain.toml`](rust-toolchain.toml)
и подхватывается автоматически. Минимальная версия для зависимостей — 1.88
(`rust-version` в корневом `Cargo.toml`).

### Windows

1. Скачайте `rustup-init.exe` с <https://rustup.rs> и запустите.
2. Для MSVC-тулчейна нужны **Visual Studio Build Tools** с компонентом
   «C++ build tools» (`VC.Tools.x86.x64` и Windows SDK) — ставятся вручную, инсталлятор rustup
   предложит ссылку.
3. Проверка в новом терминале (PowerShell или `cmd.exe`):

   ```powershell
   rustc -vV
   cargo --version
   ```

   В `rustc -vV` должно быть `host: x86_64-pc-windows-msvc`.
4. Удаление: `rustup self uninstall`.

### Linux

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc -vV && cargo --version
```

Для линковки нужен системный линкер (`build-essential` в Debian/Ubuntu, `gcc` в Fedora/Arch).
Удаление: `rustup self uninstall`.

### macOS

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
xcode-select --install   # линкер и заголовки, если ещё не установлены
rustc -vV && cargo --version
```

Удаление: `rustup self uninstall`.

### Компоненты

`rust-toolchain.toml` уже включает `rustfmt`, `clippy` и `llvm-tools-preview`.
Для отчёта о покрытии один раз ставится утилита:

```sh
cargo install cargo-llvm-cov --locked
```

## Быстрый старт

```sh
git clone https://github.com/vowatchka/pxcii.git
cd pxcii
cargo build --workspace          # первая сборка скачает крейты из crates.io
cargo run -p xtask -- gen-fixtures
cargo test -p pxcii-core
```

### Демонстрация этапа 1

```sh
cargo run -p xtask -- gen-fixtures && cargo test -p pxcii-core
```

Первая команда пересоздаёт девять фикстур 16×16 в `fixtures/`, вторая доказывает, что каждая
из них декодируется в `RgbaImage` 16×16.

### Демонстрация этапа 2

```sh
cargo run -p xtask -- convert fixtures/gradient.png --to webp
```

Читает фикстуру, конвертирует в WebP и пишет `target/convert/gradient.webp`; предупреждения
печатаются в stderr с префиксом `hint:`. Из кода:
`pxcii_core::convert(&bytes, Format::Webp, &EncodeOptions::default())?`.

### Демонстрация этапа 3

```sh
cargo run -p xtask -- gen-fixtures
cargo run -p pxcii-core --example render -- fixtures/gradient.png
```

Пример рендерит фикстуру в ANSI-псевдографику шириной 80 символов (по умолчанию).
Из кода: `render_bytes(&bytes, &RenderOptions::default())?` даёт `Grid`,
`to_ansi(&grid)` — строку с цветом, `to_text(&grid)` — монохром.

## Структура репозитория

```
pxcii-core/   # ядро: decode → render → ansi/text → encode, плюс raster (этап 5)
pxcii/        # бинарь: clap, три сабкоманды, файловый I/O (этап 4 и далее)
xtask/        # служебные команды: gen-fixtures, check-core-purity, convert
fixtures/     # по одному файлу 16×16 на каждый формат и golden/ с выходом энкодера, генерируются xtask
reviews/      # журналы ревью: PROTOCOL.md, TEMPLATE.md, stage-N.md
```

Правило границы: `pxcii-core` не знает про файловую систему, пути, переменные окружения и
процессы. Проверяется командой `cargo run -p xtask -- check-core-purity`: она падает на
`std::fs`, `std::path`, `std::env`, `fs::`, `path::`, `env::`, `process::` и на любое упоминание
`Path`. Короткие формы нужны, чтобы групповой импорт `use std::{fs, path};` не обходил правило;
совпадения ищутся с границей идентификатора, поэтому `prefs::` не считается за `fs::`.

## Разработка

Порядок работы с агентами описан в разделе 7 [`pxcii-dev-plan.md`](pxcii-dev-plan.md):
`/stage N` запускает этап, `/verify` — приёмку, `/fix F-01` — исправление конкретных замечаний,
`/models` переключает модель.

Ревью идёт по [`reviews/PROTOCOL.md`](reviews/PROTOCOL.md). Замечания, их статусы и ответы
разработчика фиксируются в `reviews/stage-N.md` (шаблон — [`reviews/TEMPLATE.md`](reviews/TEMPLATE.md)),
поэтому закрытые и отклонённые человеком замечания не выдаются повторно. Цикл
«полное ревью → правки → верификация» ограничен: simple — 1, medium — 2, complex — 3;
на исчерпанном лимите агенты останавливаются, решение принимает человек.

Агенты работают только в границах задачи: без заготовок «на будущее», попутных рефакторингов
и оптимизаций без доказанной проблемы (DRY, KISS, YAGNI, бритва Оккама). Оптимизация и
безопасность — по порогу доказательности из протокола.

Definition of Done каждого этапа (раздел 2 плана):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features
cargo llvm-cov --workspace --fail-under-lines 100
cargo run -p xtask -- check-core-purity
```

Заявленный `rust-version` (1.88) — это минимум, который требуют зависимости; его проверяет
отдельная CI-джоба, собирающая workspace на этой версии тулчейна.

Покрытие строк держится на 100% — это и есть порог DoD (`--fail-under-lines`). Покрытие
регионов ниже (около 98%): часть защитных выражений вроде `option.ok_or(ошибка)` физически не
исполняется, потому что ошибка уже отфильтрована раньше. Атрибут `#[coverage(off)]` на стабильном
Rust 1.100 недоступен, поэтому точка входа покрывается интеграционным тестом, запускающим
собранный бинарь, а не исключается из отчёта.

Правила тестов: golden-файлы и фикстуры генерируются только через `xtask gen-fixtures`; тесты
никогда не пишут в `fixtures/` — они либо сверяют закоммиченные байты, либо генерируют в `target/`
через `--out`. Сетевых фикстур и `sleep` в тестах нет. Переводы строк — LF, за это отвечает `.gitattributes`.

## Приватность

Ядро библиотеки не работает с файлами: единственный способ получить данные — передать буфер
в памяти. Запись на диск появится только как явное действие пользователя в CLI, desktop и web
(этапы 4–7); настройки интерфейса не персистятся, логи в файлы не пишутся.

## Лицензия

[MIT](LICENSE).
