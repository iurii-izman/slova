# FFmpeg Adapter — Техническая документация

## Обзор

`FfmpegAdapter` — это типизированная обёртка вокруг `ffmpeg` и `ffprobe` бинарников для:
- **Валидация видеофайлов** через `ffprobe` (проверка длительности, наличия аудиодорожки, формата)
- **Конвертация аудио** MP4 → Opus 16kHz 32kbps с опциональным шумоподавлением (`arnndn`)
- **Детектирование тишины** для чанкинга файлов >100MB
- **Нарезка аудио** по таймкодам

## Публичный API

### `probe(path: &Path) -> Result<ProbeResult, AppErrorView>`

**Назначение:** Получить метаданные видеофайла через ffprobe JSON output.

**Команда:** 
```bash
ffprobe -v quiet -print_format json -show_format -show_streams <file>
```

**Возвращает:**
```rust
pub struct ProbeResult {
    pub duration_seconds: f64,      // Общая длительность в секундах
    pub has_audio: bool,             // Есть ли аудиодорожка
    pub audio_codec: Option<String>, // Кодек аудио (e.g. "aac", "mp3")
    pub file_size_bytes: u64,        // Размер файла
    pub nb_streams: usize,           // Количество потоков
}
```

**Ошибки:**
- `INVALID_FILE: "File not found"` — файл не существует
- `INVALID_FILE: "File has no audio stream"` — нет аудиодорожки
- `INVALID_FILE: "Could not determine duration/size"` — поврежденный формат
- `INTERNAL_ERROR: "ffprobe failed: ..."` — ошибка ffprobe

---

### `extract_audio(input, output, total_duration_ms, progress_tx) -> Result<ExtractStats, AppErrorView>`

**Назначение:** Конвертировать MP4 в Opus 16kHz 32kbps с прогрессом и опциональным шумоподавлением.

**Команда:**
```bash
ffmpeg -i <input> \
  -vn -ac 1 -ar 16000 \
  -af "arnndn=m=<rnnoise_model>" \
  -c:a libopus -b:a 32k \
  -progress pipe:2 \
  <output>
```

**Параметры:**
- `input: &Path` — путь к исходному MP4 файлу
- `output: &Path` — путь для выходного Opus файла
- `total_duration_ms: u64` — общая длительность для расчета прогресса
- `progress_tx: Option<UnboundedSender<f32>>` — optional channel для отправки прогресса (0.0–1.0)

**Возвращает:**
```rust
pub struct ExtractStats {
    pub output_size_bytes: u64,      // Размер Opus файла
    pub noise_reduction_applied: bool, // Был ли применен фильтр arnndn
}
```

**Механизм прогресса:**
- ffmpeg пишет строки вида `out_time_us=<микросекунды>` в stderr
- Спавнед задача читает эти строки и отправляет прогресс как `out_time_us / 1000 / total_duration_ms`
- Если rnnoise model не найдена, фильтр `arnndn` пропускается (fallback без шумоподавления)
- Финальное значение 1.0 отправляется при успешном завершении

**Ошибки:**
- `INTERNAL_ERROR: "Failed to spawn ffmpeg"` — не удалось запустить ffmpeg
- `INTERNAL_ERROR: "ffmpeg failed with exit code ..."` — ffmpeg вернул ошибку
- `INTERNAL_ERROR: "ffmpeg produced empty output file"` — результат 0 байт
- `FS_ERROR: "Failed to stat output file"` — ошибка при чтении результата

---

### `silence_detect(audio_path: &Path) -> Result<Vec<SilencePoint>, AppErrorView>`

**Назначение:** Найти промежутки тишины в аудиофайле для чанкинга.

**Команда:**
```bash
ffmpeg -i <input> \
  -af "silencedetect=n=-40dB:d=0.5" \
  -f null - 2>&1
```

**Возвращает:**
```rust
pub struct SilencePoint {
    pub start_ms: u64,  // Начало тишины в миллисекундах
    pub end_ms: u64,    // Конец тишины в миллисекундах
}
```

**Парсинг:** Ищет в stderr строки вида:
```
[silencedetect @ ...] silence_start: 0.5
[silencedetect @ ...] silence_end: 2.5 | silence_duration: 2.0
```

---

### `cut(audio_path, start_ms, duration_ms, output) -> Result<(), AppErrorView>`

**Назначение:** Вырезать отрезок аудио без re-encoding (быстро).

**Команда:**
```bash
ffmpeg -ss <start_sec> -i <input> \
  -t <duration_sec> \
  -c copy <output>
```

**Параметры:**
- Временные параметры автоматически конвертируются из миллисекунд в секунды

---

## Архитектурные решения

### 1. Progress через channel вместо callback

**Почему:** Замыкания с `FnMut` слабо работают с `move` в async контексте. Использование `tokio::sync::mpsc::UnboundedSender` позволяет:
- Нет lifetime issues
- Легче юнит-тестировать (мок-канал)
- Интеграция с `JobScheduler` через event loop

### 2. Условное включение arnndn фильтра

```rust
let has_rnnoise = self.rnnoise_model.exists();
if !has_rnnoise {
    // fallback без фильтра, но с успехом
}
```

**Почему:** Если пользователь не загрузил rnnoise model, не падаем, а продолжаем работу.

### 3. Парсинг stderr вместо json

- `ffmpeg -progress pipe:2` пишет сырые строки (не JSON)
- Парсим построчно для real-time прогресса
- Надежнее чем регулярные выражения

### 4. Async spawn для progress reader

```rust
let progress_handle = tokio::spawn(async move { ... });
// ... ffmpeg работает в параллели с reader
let _ = tokio::time::timeout(1s, progress_handle).await;
```

**Почему:** 
- Не блокируем основной тред ввода-вывода
- reader может отстать (и это OK)
- timeout страхует от зависания

---

## Тестирование

### Unit тесты на фиксчурах

Все тесты используют строковые фиксчуры, **без реального ffmpeg**:

```bash
cargo test --bin slova-tauri adapters::ffmpeg::tests
```

Тесты:
- `test_parse_ffprobe_output` — парсинг JSON с видео + аудиодорожками
- `test_parse_ffprobe_minimal` — парсинг минимального JSON
- `test_parse_progress_output` — парсинг `out_time_us=...`
- `test_progress_calculation` — расчет прогресса 0.0–1.0
- `test_parse_silence_output` — парсинг ffmpeg silencedetect stderr
- `test_probe_result` — конструирование ProbeResult
- `test_extract_stats` — конструирование ExtractStats

### Интеграционные тесты (TODO)

Нужны тесты с реальными видеофайлами:
- `test_probe_real_mp4` (требует тестовый MP4)
- `test_extract_audio_real` (требует ffmpeg в PATH)

---

## Интеграция с JobScheduler

```rust
let ffmpeg = FfmpegAdapter::new(
    ffmpeg_path,
    ffprobe_path,
    rnnoise_model_path,
);

// В state machine job:
let probe_result = ffmpeg.probe(&job.source_path).await?;

let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
let stats = ffmpeg.extract_audio(
    &input_path,
    &output_path,
    (probe_result.duration_seconds * 1000.0) as u64,
    Some(tx),
).await?;

// rx в отдельной задаче отправляет progress в UI через Tauri events
```

---

## Безопасность

### Пути и символы

- Все пути передаются через `&Path` (typesafe)
- ffmpeg запускается через `Command::new()` + `.arg()`, **не через shell**
- Windows paths с пробелами и кириллицей обрабатываются правильно

### Secrets

- API ключи **не логируются** (находятся в OS keychain)
- Транскрипты **не логируются** полностью (в разработке только первые 100 chars)

### Ресурсы

- Progress reader спавнится в отдельной задаче с timeout 1 сек
- Если ffmpeg завис, не зависнет весь scheduler
- Временные файлы удаляются в JobScheduler при cancellation

---

## Ошибки и обработка

Все методы возвращают `Result<T, AppErrorView>`, где `AppErrorView` — типизированная ошибка:

```rust
pub struct AppErrorView {
    pub code: String,             // e.g. "INVALID_FILE"
    pub message: String,
    pub details: Option<String>,  // extra context
}
```

UI показывает `message`, логирует `code + details`.

---

## Что осталось (Post-MVP)

1. **Cancellation token**: Добавить `CancellationToken` параметр в `extract_audio` для прерывания ffmpeg
2. **Интеграция с Groq**: Передать результат `extract_audio` (Opus) в Groq API
3. **Fallback-чанкинг**: Если output >100MB, использовать `silence_detect` для нарезки
4. **Кэширование**: По SHA256 хешу файла проверять, обрабатывался ли уже
5. **Логирование**: Добавить структурированные логи через `tracing`

---

## Ссылки

- `src-tauri/src/adapters/ffmpeg.rs` — реализация
- `transcriber-spec.md` — требования к FFmpeg pipeline
- `transcriber-architecture-analysis.md` — архитектура JobScheduler
