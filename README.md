# pxcii

Конвертер изображений в псевдографику: один бинарь, три режима (CLI, desktop, web), плюс
Python-обёртка. Ядро — чистый Rust без C-зависимостей, весь пайплайн работает в памяти.

## Статус

Идёт разработка по этапам, источник истины — [`pxcii-dev-plan.md`](pxcii-dev-plan.md),
требования — [`pxcii-tz.md`](pxcii-tz.md).

| Этап | Состояние |
|---|---|
| 1. Фундамент + чтение форматов | **готов**: библиотека декодирует 9 форматов |
| 2. Запись форматов + конвертация | не начат |
| 3. Ядро рендера | не начат |
| 4. CLI | не начат |
| 5. Растеризация | не начат |
| 6. Desktop | не начат |
| 7. Web | не начат |
| 8. Python-пакет | не начат |
| 9. Релиз-обвязка | не начат |

На этом этапе `pxcii` — заглушка, которая печатает баннер: CLI появляется на этапе 4.
Полезная функциональность пока живёт в библиотеке `pxcii-core`.

## Поддерживаемые форматы

Вход (этап 1) и, начиная с этапа 2, выход: **PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, TGA, PNM**.

Ограничения, принятые осознанно:

- многокадровые GIF/TIFF/WebP читаются по первому кадру;
- вход ограничен 100 000 000 пикселей, лимит проверяется по заголовку файла;
- HEIC, AVIF, SVG и QOI не поддерживаются — требуют C-библиотек или вне задачи.

TGA не имеет сигнатуры, поэтому определяется эвристикой по 18-байтовому заголовку с откатом
на расширение файла. TIFF декодируется в моделях gray, RGB и RGBA; палитровые файлы и
gray+alpha возвращают `CorruptImage` — это осознанное ограничение первого этапа.

Переводы строк задаёт `.gitattributes` (`eol=lf`), поэтому настраивать `core.autocrlf` не нужно.
Только на Windows с длинными путями может понадобиться локальная настройка репозитория:
`git config --local core.longpaths true` (глобальный конфиг не требуется).

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

## Структура репозитория

```
pxcii-core/   # ядро: decode → (этап 3) render → ansi/text/raster → (этап 2) encode
pxcii/        # бинарь: clap, три сабкоманды, файловый I/O (этап 4 и далее)
xtask/        # служебные команды: gen-fixtures, check-core-purity
fixtures/     # по одному файлу 16×16 на каждый формат, генерируются xtask
```

Правило границы: `pxcii-core` не знает про файловую систему, пути, переменные окружения и
процессы. Проверяется командой `cargo run -p xtask -- check-core-purity`: она падает на
`std::fs`, `std::path`, `std::env`, `fs::`, `path::`, `env::`, `process::` и на любое упоминание
`Path`. Короткие формы нужны, чтобы групповой импорт `use std::{fs, path};` не обходил правило;
совпадения ищутся с границей идентификатора, поэтому `prefs::` не считается за `fs::`.

## Разработка

Порядок работы с агентами описан в разделе 7 [`pxcii-dev-plan.md`](pxcii-dev-plan.md):
`/stage N` запускает этап, `/verify` — приёмку, `/models` переключает модель.

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
