# Changelog

Формат: [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версии по
[Semantic Versioning](https://semver.org/lang/ru/).

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
