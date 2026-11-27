# Changelog

Формат: [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версии по
[Semantic Versioning](https://semver.org/lang/ru/).

## [Не выпущено]

Третий этап: ядро рендера в псевдографику.

### Добавлено

- `pxcii_core::render_bytes(&[u8], &RenderOptions) -> Grid`: декодирование и раскладка
  изображения по сетке символов.
- `pxcii_core::RenderOptions` (`cols`, `color`, `ramp`, `dither`, `cell_aspect`),
  `ColorMode` (`TrueColor`, `Ansi256`, `Mono`), `Ramp` (`Ascii`, `Blocks`, `Braille`,
  `Custom`), `Dither` (`None`, `FloydSteinberg`, `Ordered`).
- `pxcii_core::to_ansi(&Grid) -> String` и `pxcii_core::to_text(&Grid) -> String`.
- Box-фильтр на `f32` для приведения к сетке, яркость через линейный свет без `powf`.
- Braille: ячейка 2×4 точки, побитовая маска в кодпоинт U+2800…U+28FF.
- Целочисленный Floyd–Steinberg и Bayer 8×8: вывод детерминирован на всех платформах.
- Golden-гриды `fixtures/golden/grids/*.txt` для шести конфигураций, генерация —
  `xtask gen-fixtures`; демо — `pxcii-core/examples/render.rs`.

Второй этап: запись всех девяти форматов и конвертация.

### Добавлено

- `pxcii_core::encode(&RgbaImage, Format, &EncodeOptions) -> Encoded`: PNG, JPEG (4:4:4,
  качество 100 по умолчанию), WebP (lossless), GIF (256 цветов, Floyd–Steinberg), BMP, TIFF
  (deflate), ICO, TGA, PNM (P6). Результат детерминирован.
- `pxcii_core::Warning` (`IcoDownscaled`, `GifQuantized`, `JpegArtifacts`, `AlphaDropped`):
  ограничения формата сообщаются в `Encoded::warnings`, а не замалчиваются.
- `pxcii_core::convert(bytes, Format, &EncodeOptions) -> Converted`: декодирование и запись за
  один вызов.
- Box-фильтр на целых числах для уменьшения ICO до 256 px.
- Зависимости `jpeg-encoder` и `color_quant` (оба pure Rust).
- `xtask gen-fixtures` пишет golden-файлы `fixtures/golden/` для шести lossless-форматов;
  `xtask convert <файл> --to <формат>` — демонстрация этапа.

## [0.1.0] — 2026-11-25

Первый этап: фундамент репозитория и чтение всех девяти форматов.

### Добавлено

- Workspace из трёх крейтов: `pxcii-core` (ядро), `pxcii` (бинарь-заглушка),
  `xtask` (служебные команды); `pxcii-py` появится на этапе 8.
- `pxcii_core::Format` — перечисление девяти форматов, `from_ext`, `sniff` по magic bytes,
  `mime`, `extension`, `name`.
- `pxcii_core::decode` / `decode_with_hint` — декодирование PNG, JPEG, WebP, GIF, BMP, TIFF, ICO,
  TGA и PNM в `RgbaImage`; только первый кадр у многокадровых файлов.
- Ориентация EXIF у JPEG: ручной разбор тега `0x0112` и все восемь преобразований.
- `pxcii_core::PxciiError`: `UnsupportedFormat`, `CorruptImage`, `DimensionLimit`, `Encode`, `Io`.
- Лимит `MAX_PIXELS = 100_000_000`, проверяется по заголовку файла до выделения буфера.
- `xtask gen-fixtures` — программная генерация девяти фикстур 16×16 и
  `xtask check-core-purity` — запрет `std::fs`, `Path` и `env::` в ядре.
- CI на GitHub Actions: `fmt`, `clippy`, `test` (три ОС), покрытие строк с артефактом `lcov`,
  проверка чистоты ядра.
- Документация: `README.md` с установкой Rust и порядком запуска, `LICENSE` (MIT).
