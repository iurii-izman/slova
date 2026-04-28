# VideoTranscriber — детальный архитектурный анализ и вариант реализации

Дата подготовки: 2026-04-28

## 1. Что важного в документе

**Жёсткие ограничения, на которые опирается всё остальное:**

- Железо: Ryzen 3 / 8 GB / Win11, без GPU → любая локальная Whisper-модель (CTranslate2/whisper.cpp) отваливается, ставка на cloud-only.
- Объём: 10–50 файлов, средний звук с шумом, RU.
- Приоритеты: **Скорость > Точность(RU) > Cost** — это фактически делает выбор Groq Whisper-Large-v3-Turbo предопределённым (216×, $0.04/ч).

**Ключевые архитектурные решения, заложенные автором:**

1. Tauri + Solid.js + Rust — нативное окно, маленький бинарь, реактивный UI.
2. Pipeline: `ffprobe → ffmpeg(Opus 16kHz mono 32kbps + arnndn) → Groq → verbose_json → .txt`.
3. Параллельность: `tokio::Semaphore(3)` под free tier (30 rpm).
4. Fallback-чанкинг по тишине с overlap 5–10 сек, склейка по таймкодам.
5. `verbose_json` как источник истины — сразу даёт текст, сегменты, таймкоды (под SRT/JSON/склейку).
6. SQLite-история + кэш по хешу + keychain для API-ключа.
7. State machine: `queued → extracting → uploading → transcribing → done|error`.

**Числовые ориентиры (важны для проектирования UX):**

- 1 файл (30 мин видео) ≈ 15–25 сек end-to-end.
- 5 файлов параллельно ≈ 45–60 сек.
- ffmpeg ~5–7 сек/30 мин CPU, аплоад ~1–2 сек, Groq ~8–15 сек.

---

## 2. Что в спецификации сильно, а что — пробел

### Сильные стороны

- Выбор Opus@16kHz mono — реально оптимум для STT, не теоретический.
- `temperature=0` + `prompt` — это две главные ручки против галлюцинаций Whisper, и они учтены.
- `verbose_json` как единственный формат — правильно: позже легко получить SRT/JSON/plain text без повторного запроса.
- Идея «без чанкинга по умолчанию» — критична для качества пунктуации, чанкинг только как fallback.

### Что я бы пересмотрел или дополнил

| # | Проблема в спеке | Решение |
|---|---|---|
| 1 | `Semaphore(3)` один на всё. Но ffmpeg — CPU-bound, Groq — network-bound. На Ryzen 3 параллельный ffmpeg задушит систему. | **Два независимых семафора**: `cpu_sem(2)` для ffmpeg/ffprobe, `net_sem(3)` для Groq. |
| 2 | Не описана **отмена** задачи (пользователь жмёт «Cancel» в середине транскрибации). | `tokio_util::sync::CancellationToken` на каждую задачу + AbortHandle для reqwest. |
| 3 | `arnndn=m=rnnoise-models/cb.rnnn` — модель шумоподавления нужно где-то хранить и поставлять. Без неё фильтр упадёт. | Бандлить `cb.rnnn` в `resources/`, путь резолвить через Tauri resource resolver. |
| 4 | Прогресс ffmpeg не описан, хотя пользователь хочет «реальный %» на стадии extract тоже. | Парсить stderr ffmpeg по `out_time_us=` через `-progress pipe:2`. |
| 5 | Прогресс аплоада через reqwest stream — упомянуто, но без деталей. | Обёртка `ProgressBody` над `reqwest::Body::wrap_stream` + `multipart` через `Part::stream_with_length`. |
| 6 | Кэш «по хешу файла» — для 200 MB MP4 SHA-256 = ~1 сек на SSD, заметная задержка перед стартом. | Использовать **BLAKE3** и weak-key для быстрой дедупликации в рамках батча. |
| 7 | Чанкинг описан рамочно: «склейка по таймкодам, дедупликация». Это самая багогенная часть. | Детальный алгоритм через сравнение последних N токенов чанка K с первыми токенами чанка K+1 в зоне overlap. |
| 8 | SQLite-схема не определена. | Задать конкретную схему: `jobs`, `transcripts`, `cache`. |
| 9 | Не сказано про race condition при write: что если `video.txt` уже существует или открыт во внешнем редакторе? | Атомарная запись: `video.txt.tmp` → `fs::rename`. Стратегия конфликтов: suffix/overwrite/skip. |
| 10 | Что делать с папкой при D&D, а не файлами? | Рекурсивный walker (`walkdir`) с фильтром по расширениям и `ffprobe`-проверкой. |
| 11 | UI при 50 файлах в очереди начнёт лагать, если рендерить всё. | Виртуальный список. |
| 12 | Tauri events на каждый прогресс-тик легко зальют main-thread. | Throttle прогресса до ≤10 Hz на задачу + батчинг. |
| 13 | Безопасность: ffmpeg-args собираются из путей пользователя. | Никаких shell-строк — только argv-форма через `Command::arg`. |
| 14 | Logging/observability не упомянуты. | `tracing` + rolling-файл в `%APPDATA%\VideoTranscriber\logs\`. |
| 15 | Auto-update упомянут «после MVP», но генерация ключей и подписи лучше предусмотреть до релиза. | Сразу заложить секцию updater в конфиге Tauri. |

---

## 3. Целевая архитектура

### 3.1 Слои

```text
┌─────────────────────────────────────────────────────────────────┐
│  Solid.js UI  (apps/ui)                                         │
│   ─ Pages: Queue, Detail, Settings                              │
│   ─ Stores: queueStore, settingsStore, logsStore (Solid stores) │
│   ─ IPC adapter: tauri.invoke / listen                          │
└──────────────┬──────────────────────────────────────────────────┘
               │  Tauri commands + events (typed via specta)
┌──────────────▼──────────────────────────────────────────────────┐
│  Tauri host (src-tauri/src/app)                                 │
│   ─ commands.rs: thin handlers, only validation + dispatch      │
│   ─ events.rs:   typed event emitter (throttled)                │
│   ─ AppState:    Arc<Inner> с очередью, БД, секретами           │
└──────────────┬──────────────────────────────────────────────────┘
               │  channels (mpsc) + Arc<Mutex>
┌──────────────▼──────────────────────────────────────────────────┐
│  Core domain  (src-tauri/src/core)                              │
│   ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐   │
│   │ JobScheduler │→ │  Pipeline    │→ │ Result writer (fs)  │   │
│   │  (tokio)     │  │  state-mach. │  │                     │   │
│   └──────┬───────┘  └──────┬───────┘  └─────────────────────┘   │
│          │                 │                                    │
│   ┌──────▼─────┐    ┌──────▼─────┐   ┌──────────────────────┐   │
│   │ CpuSem(2)  │    │ NetSem(3)  │   │  Retry/backoff       │   │
│   └────────────┘    └────────────┘   └──────────────────────┘   │
└──────────────┬──────────────────────────────────────────────────┘
               │
┌──────┬───────┴─────────┬──────────────┬───────────────┬─────────┐
│ FFmp │  Groq HTTP      │  SQLite      │  Keyring      │ Logger  │
│ adap │  (whisper+llm)  │  (sqlx)      │  (windows-cm) │(tracing)│
└──────┴─────────────────┴──────────────┴───────────────┴─────────┘
```

### 3.2 Структура репозитория

```text
slova/
├─ src-tauri/                       # Rust backend + Tauri host
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  ├─ build.rs
│  ├─ binaries/                     # ffmpeg, ffprobe sidecars (per OS)
│  │   └─ ffmpeg-x86_64-pc-windows-msvc.exe
│  ├─ resources/
│  │   └─ rnnoise-models/cb.rnnn    # шумодав
│  └─ src/
│      ├─ main.rs                   # bootstrap (logger, AppState, builder)
│      ├─ app/
│      │   ├─ mod.rs
│      │   ├─ commands.rs           # #[tauri::command]
│      │   ├─ events.rs             # typed emitter + throttle
│      │   └─ state.rs              # AppState, init
│      ├─ core/
│      │   ├─ mod.rs
│      │   ├─ scheduler.rs          # JobScheduler
│      │   ├─ pipeline.rs           # state machine, orchestration
│      │   ├─ stages/
│      │   │   ├─ probe.rs          # ffprobe
│      │   │   ├─ extract.rs        # ffmpeg → opus
│      │   │   ├─ chunk.rs          # silencedetect + cutting
│      │   │   ├─ upload.rs         # multipart с прогрессом
│      │   │   ├─ transcribe.rs     # Groq Whisper
│      │   │   ├─ stitch.rs         # склейка чанков
│      │   │   ├─ postprocess.rs    # Groq Llama (опц.)
│      │   │   └─ write_result.rs   # txt/srt/json
│      │   ├─ retry.rs              # exponential backoff
│      │   ├─ cancel.rs             # CancellationToken helpers
│      │   └─ progress.rs           # ProgressBus + Throttle
│      ├─ adapters/
│      │   ├─ ffmpeg.rs             # типобезопасная обёртка sidecar
│      │   ├─ groq/
│      │   │   ├─ mod.rs
│      │   │   ├─ whisper.rs
│      │   │   ├─ llama.rs
│      │   │   └─ types.rs          # serde-структуры verbose_json
│      │   ├─ keyring.rs
│      │   └─ fs.rs                 # атомарная запись, hash
│      ├─ db/
│      │   ├─ mod.rs                # sqlx pool + миграции
│      │   ├─ migrations/
│      │   │   ├─ 0001_init.sql
│      │   │   └─ 0002_cache.sql
│      │   └─ repo.rs               # JobRepo, CacheRepo
│      ├─ types/
│      │   ├─ mod.rs                # Job, JobState, Settings, ...
│      │   └─ errors.rs             # AppError (thiserror)
│      └─ telemetry.rs              # tracing init
│
├─ apps/ui/                         # Solid.js frontend
│  ├─ package.json
│  ├─ vite.config.ts
│  ├─ index.html
│  └─ src/
│      ├─ main.tsx
│      ├─ App.tsx
│      ├─ ipc/
│      │   ├─ commands.ts           # обёртки invoke (typed)
│      │   ├─ events.ts             # listen + Solid store sync
│      │   └─ bindings.ts           # сгенерировано specta/tauri-bindgen
│      ├─ stores/
│      │   ├─ queue.store.ts
│      │   ├─ settings.store.ts
│      │   └─ toasts.store.ts
│      ├─ pages/
│      │   ├─ Queue/
│      │   │   ├─ Queue.tsx
│      │   │   ├─ DropZone.tsx
│      │   │   ├─ JobRow.tsx
│      │   │   └─ VirtualList.tsx
│      │   ├─ Detail/Detail.tsx     # превью + редактор текста
│      │   └─ Settings/Settings.tsx
│      ├─ components/               # Button, ProgressBar, Toast, ...
│      └─ lib/
│          ├─ format.ts             # форматирование длительностей, %
│          └─ keymap.ts             # горячие клавиши
│
├─ docs/
│  ├─ transcriber-spec.md           # текущий
│  ├─ architecture.md               # этот документ
│  └─ adr/                          # architectural decision records
└─ .github/workflows/               # CI: build, lint, test, release
```

---

## 4. Детальные решения по слоям

### 4.1 Доменные типы

```rust
#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct JobId(pub Uuid);

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub enum JobState {
    Queued,
    Probing,
    Extracting   { progress: f32 },
    Chunking     { progress: f32 },
    Uploading    { progress: f32, chunk_idx: u32, chunk_total: u32 },
    Transcribing { chunk_idx: u32, chunk_total: u32 },
    Stitching,
    Postprocessing,
    Done         { output_path: PathBuf, duration_ms: u64 },
    Failed       { error: AppErrorView, attempts: u32 },
    Cancelled,
    Paused,
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct Job {
    pub id: JobId,
    pub source_path: PathBuf,
    pub display_name: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub state: JobState,
    pub settings_snapshot: JobSettings,
    pub content_hash: Option<[u8; 32]>,
}
```

`specta` или `tauri-specta` автогенерит `bindings.ts` — фронт и бэк используют одни и те же типы.

### 4.2 Команды Tauri

```rust
#[tauri::command]
async fn enqueue_files(state: State<'_, AppState>, paths: Vec<PathBuf>)
    -> Result<Vec<JobId>, AppErrorView>;

#[tauri::command]
async fn cancel_job(state: State<'_, AppState>, id: JobId) -> Result<(), AppErrorView>;

#[tauri::command]
async fn retry_job(state: State<'_, AppState>, id: JobId) -> Result<(), AppErrorView>;

#[tauri::command]
async fn pause_queue(state: State<'_, AppState>) -> Result<(), AppErrorView>;

#[tauri::command]
async fn resume_queue(state: State<'_, AppState>) -> Result<(), AppErrorView>;

#[tauri::command]
async fn list_jobs(state: State<'_, AppState>, filter: JobFilter)
    -> Result<Vec<Job>, AppErrorView>;

#[tauri::command]
async fn get_transcript(state: State<'_, AppState>, id: JobId)
    -> Result<Transcript, AppErrorView>;

#[tauri::command]
async fn save_transcript_edit(state: State<'_, AppState>, id: JobId, text: String)
    -> Result<(), AppErrorView>;

#[tauri::command]
async fn export(state: State<'_, AppState>, id: JobId, format: ExportFormat)
    -> Result<PathBuf, AppErrorView>;

#[tauri::command]
async fn save_api_key(key: String) -> Result<(), AppErrorView>;

#[tauri::command]
async fn get_settings() -> Result<Settings, AppErrorView>;

#[tauri::command]
async fn set_settings(s: Settings) -> Result<(), AppErrorView>;
```

### 4.3 События

Один батчевый канал событий, чтобы не зашумлять main-thread:

```rust
#[derive(Serialize, specta::Type)]
struct QueueTick {
    updates: Vec<JobUpdate>,
    ts: u64,
}

#[derive(Serialize, specta::Type)]
struct JobUpdate {
    id: JobId,
    state: JobState,
    bytes_uploaded: Option<u64>,
    eta_ms: Option<u64>,
}
```

Редкие события можно держать отдельно: `job:done`, `job:failed`, `job:cancelled`, `queue:idle`, `app:error`, `app:rate-limited`.

### 4.4 Планировщик и параллельность

```rust
pub struct JobScheduler {
    cpu_sem: Arc<Semaphore>,
    net_sem: Arc<Semaphore>,
    rate_limit: Arc<RateLimit>,
    cancels: DashMap<JobId, CancellationToken>,
    progress_tx: mpsc::Sender<JobUpdate>,
    repo: Arc<JobRepo>,
    ffmpeg: Arc<FfmpegAdapter>,
    groq: Arc<GroqClient>,
}
```

Почему два семафора, а не один:

- ffmpeg на Ryzen 3 при `-threads 0` уже использует все ядра.
- 3 параллельных ffmpeg = деградация всей системы и UI.
- Сетевой запрос к Groq не должен блокироваться CPU-семафором.
- Между этапами задача освобождает `cpu_sem` сразу после конвертации и приобретает `net_sem` перед аплоадом.

Дополнительно нужен глобальный rate-limiter (`governor` crate) перед `net_sem.acquire()`, чтобы не пробить 30 rpm.

### 4.5 Pipeline как state machine

```rust
async fn run(job: Job, ctx: PipelineCtx, cancel: CancellationToken) -> Result<()> {
    set_state(&job, JobState::Probing).await;
    let probe = stages::probe::run(&job.source_path).await?;
    if !probe.has_audio { bail!(NoAudioTrack); }

    if let Some(prev) = ctx.cache.lookup(&job).await? {
        return reuse_cached(prev, &job).await;
    }

    let _cpu = ctx.cpu_sem.acquire().await?;
    let opus = stages::extract::run(&job, &probe, &ctx, &cancel).await?;
    drop(_cpu);

    let chunks = if opus.size > 100 * MB {
        stages::chunk::run(&opus, &ctx, &cancel).await?
    } else {
        vec![Chunk::single(opus)]
    };

    let mut handles = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let permit = ctx.net_sem.clone().acquire_owned().await?;
        let h = tokio::spawn(stages::transcribe::run(
            chunk.clone(), i, ctx.clone(), cancel.clone(), permit
        ));
        handles.push(h);
    }
    let parts = try_join_all(handles).await?;

    let transcript = stages::stitch::run(parts, &chunks)?;

    let final_text = if ctx.settings.postprocess {
        stages::postprocess::run(&transcript, &ctx).await?
    } else {
        transcript
    };

    stages::write_result::run(&job, &final_text, &ctx.settings).await?;
    ctx.cache.store(&job, &final_text).await?;
    set_state(&job, JobState::Done { .. }).await;
    Ok(())
}
```

Ключевые свойства:

- Каждое `await?` проверяется через `tokio::select!` с `cancel.cancelled()`.
- Любая ошибка классифицируется retry-слоем.
- Состояние пишется в БД на каждом переходе.

### 4.6 Retry / backoff / классификация ошибок

| Класс | Примеры | Retry? | Стратегия |
|---|---|---|---|
| `Validation` | нет аудиодорожки, повреждённый MP4 | ❌ | сразу Failed |
| `LocalIO` | нет места на диске, нет прав | ❌ | сразу Failed, понятная подсказка |
| `Transient(Network)` | timeout, connection reset | ✅ | exp backoff: 1s, 2s, 4s, 8s, max 30s |
| `Transient(RateLimit)` | 429, `Retry-After` header | ✅ | спать ровно `Retry-After` + jitter |
| `Transient(Server)` | 5xx | ✅ | exp backoff, max 5 попыток |
| `Auth` | 401 | ❌ | Failed + событие `app:auth-failed` |
| `Cancelled` | пользователь | — | специальный финальный стейт |

```rust
let mut attempt = 0;
loop {
    match op().await {
        Ok(v) => return Ok(v),
        Err(e) if !e.is_transient() || attempt >= max => return Err(e),
        Err(e) => {
            let delay = match &e {
                AppError::RateLimited { retry_after } => *retry_after,
                _ => Duration::from_millis(500 * (1u64 << attempt))
                       .min(Duration::from_secs(30)),
            };
            let jitter = rand::thread_rng().gen_range(0..200);
            tokio::time::sleep(delay + Duration::from_millis(jitter)).await;
            attempt += 1;
        }
    }
}
```

### 4.7 Чанкинг и склейка

**Когда:** только если после ffmpeg размер > 100 MB. На 32 kbps это примерно 7 часов непрерывного аудио, поэтому кейс редкий, но обязательный.

**Алгоритм нарезки:**

1. Запускаем `ffmpeg -i a.opus -af silencedetect=noise=-30dB:d=0.5 -f null -`.
2. Парсим stderr, получаем интервалы тишины.
3. Идём по аудио и режем в середине ближайшей тишины так, чтобы каждый чанк был ≤ ~80 MB.
4. Добавляем overlap 5 секунд до и после соседнего чанка.
5. Сохраняем `chunk_i.opus` с метаданными `{ start_global, end_global, overlap_pre, overlap_post }`.

**Алгоритм склейки:**

1. Для каждого чанка получаем `verbose_json.segments[]` с локальными таймкодами.
2. Сдвигаем все локальные таймкоды на `start_global` чанка → глобальные.
3. Между чанками K и K+1 есть зона перекрытия.
4. В этой зоне вероятны повторные слова.
5. Дедупликация: берём последние `M` сегментов чанка K, сравниваем с первыми `M` сегментами чанка K+1.
6. Используем выравнивание по таймкоду + текстовое сходство: нормализованная Левенштейн-близость на токенах ≥ 0.7.
7. Точка склейки выбирается как минимум по сумме «расстояние по времени + 1 - сходство по тексту».
8. Всё, что в K было после точки склейки, и всё, что в K+1 было до неё, выкидывается.

### 4.8 SQLite-схема

```sql
CREATE TABLE jobs (
    id              TEXT PRIMARY KEY,
    source_path     TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    content_hash    BLOB,
    created_at      INTEGER NOT NULL,
    finished_at     INTEGER,
    state           TEXT NOT NULL,
    state_payload   TEXT,
    output_path     TEXT,
    settings_json   TEXT NOT NULL,
    attempts        INTEGER NOT NULL DEFAULT 0,
    error_message   TEXT,
    error_code      TEXT
);

CREATE INDEX idx_jobs_state    ON jobs(state);
CREATE INDEX idx_jobs_created  ON jobs(created_at DESC);
CREATE INDEX idx_jobs_hash     ON jobs(content_hash);

CREATE TABLE transcripts (
    job_id          TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
    plain_text      TEXT NOT NULL,
    segments_json   TEXT NOT NULL,
    edited_text     TEXT,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE cache (
    cache_key       TEXT PRIMARY KEY,
    job_id          TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    created_at      INTEGER NOT NULL
);
```

### 4.9 FFmpeg-обёртка

```rust
pub struct FfmpegAdapter {
    exe: PathBuf,
    ffprobe: PathBuf,
    rnnoise_model: PathBuf,
}

impl FfmpegAdapter {
    pub async fn probe(&self, p: &Path) -> Result<ProbeResult> { /* ffprobe -of json */ }

    pub async fn extract_audio(
        &self,
        input: &Path,
        output: &Path,
        cancel: &CancellationToken,
        progress: impl FnMut(f32) + Send + 'static,
    ) -> Result<ExtractStats> {
        // -progress pipe:2 → парсим out_time_us, total_duration → 0.0..1.0
    }

    pub async fn silence_detect(&self, input: &Path) -> Result<Vec<SilenceSpan>>;

    pub async fn cut(&self, input: &Path, span: TimeSpan, output: &Path) -> Result<()>;
}
```

### 4.10 Groq-клиент

```rust
pub struct GroqClient {
    http: reqwest::Client,
    api_key: SecretString,
    base_url: Url,
}

impl GroqClient {
    pub async fn transcribe(
        &self,
        audio: &Path,
        opts: TranscribeOpts,
        on_upload: impl Fn(u64, u64) + Send + Sync + 'static,
        cancel: &CancellationToken,
    ) -> Result<VerboseJson> { ... }
}

#[derive(Default)]
pub struct TranscribeOpts {
    pub language: Option<String>,
    pub temperature: f32,
    pub prompt: Option<String>,
    pub model: &'static str,
    pub response_format: Format,
}
```

Реализация прогресса аплоада: `reqwest::multipart::Part::stream_with_length(body, len)` поверх `tokio_util::io::ReaderStream`, обёрнутого в счётчик прогресса. API-ключ хранится в `keyring` и в памяти представлен как `secrecy::SecretString`.

### 4.11 Кэш

Ключ: `BLAKE3(file_bytes) + BLAKE3(settings_fingerprint)`.

Рекомендация:

- Для полной надёжности хранить полный BLAKE3.
- Для быстрой дедупликации в текущем батче использовать weak-key: `(size, mtime, head_4M_hash)`.
- Если пользователь меняет prompt/язык/модель — `settings_fingerprint` меняется, файл обрабатывается заново.

### 4.12 Запись результата

```rust
async fn write_atomic(target: &Path, data: &[u8], conflict: ConflictPolicy) -> Result<PathBuf> {
    let final_path = match conflict {
        ConflictPolicy::Overwrite => target.to_path_buf(),
        ConflictPolicy::Suffix    => unique_with_suffix(target).await?,
        ConflictPolicy::Skip      => {
            if target.exists() { return Err(AlreadyExists) }
            target.to_path_buf()
        },
    };
    let tmp = final_path.with_extension("tmp");
    fs::write(&tmp, data).await?;
    fs::rename(&tmp, &final_path).await?;
    Ok(final_path)
}
```

### 4.13 Cancellation и Pause

- На задачу — `CancellationToken`.
- Дочерние чанки наследуют token задачи.
- `Cancel job` отменяет токен, текущий ffmpeg прибивается через `child.start_kill()`, активный `reqwest` роняется через `tokio::select!`.
- `Pause queue` не отменяет активные задачи, просто перестаёт пускать новые из `Queued`.
- После отмены обязательно чистятся временные файлы (`*.opus`, `*.tmp`).

---

## 5. UI/UX архитектура

### 5.1 Store очереди в Solid

```ts
type QueueState = {
  jobs: Record<JobId, Job>;
  order: JobId[];
  filter: 'all' | 'active' | 'failed' | 'done';
};

const [queue, setQueue] = createStore<QueueState>({ jobs: {}, order: [], filter: 'all' });

listen<QueueTick>('queue:tick', ev => {
  setQueue('jobs', produce(jobs => {
    for (const u of ev.payload.updates) {
      jobs[u.id] = { ...jobs[u.id], state: u.state };
    }
  }));
});
```

Solid с `createStore + produce` обновляет точечно. При >50 задачах включаем виртуализацию.

### 5.2 Состояние UI каждой строки

| Бэкенд-стейт | Что показываем |
|---|---|
| `Queued` | серый чип «В очереди» |
| `Probing` | spinner |
| `Extracting{0..1}` | progress bar |
| `Chunking` | progress bar |
| `Uploading{p, i, total}` | progress bar + «чанк i/total», скорость МБ/с, ETA |
| `Transcribing` | indeterminate, текст «Транскрибация…» |
| `Stitching` / `Postprocessing` | spinner |
| `Done` | checkmark, кнопки «Открыть», «Скопировать», «Экспорт» |
| `Failed` | красный, Retry, раскрывающаяся подсказка по ошибке |
| `Cancelled` | серый, Retry |

### 5.3 Drag & Drop

- Drop-zone для нескольких файлов.
- Если пользователь дропнул папку — рекурсивный walk на бэке.
- Дубликаты в текущем батче схлопываются по weak-key.

### 5.4 Detail / Edit

- При выборе задачи `get_transcript` возвращает `plain_text + segments + edited_text`.
- MVP: простой `<textarea>` с автосохранением через debounce 500 мс.
- Сегменты с таймкодами показываются в боковой колонке как кликабельные якоря.

### 5.5 Settings

- API key: маска, кнопка «Проверить».
- Язык: `ru` по умолчанию, можно auto.
- Prompt: textarea, дефолт из спецификации.
- Format: txt / srt / json.
- Параллельность: `net_sem` 1..5, `cpu_sem` 1..N.
- Conflict policy: overwrite / suffix / skip.
- Postprocess через Llama: on/off + выбор модели.
- Output: рядом с исходником / в указанную папку.
- Auto-update: on/off после MVP.

---

## 6. Безопасность

1. API-ключ только в Windows Credential Manager через `keyring`.
2. В БД и логах ключа нет. Для дебага — маска `gsk_***1234`.
3. `secrecy::SecretString` + `zeroize` для ключа в памяти.
4. Никаких shell-строк для ffmpeg — только argv.
5. Path traversal: при атомарной записи проверять, что final path внутри ожидаемой директории.
6. CSP в Tauri — строгая, без `unsafe-eval`, без лишних внешних доменов.
7. Tauri 2 capabilities — выдавать только нужные разрешения.
8. Логи не должны содержать содержимое транскрипта.
9. HTTPS only, проверка TLS сертификатов по умолчанию у reqwest.

---

## 7. Тестирование и наблюдаемость

### 7.1 Unit

- `retry.rs` — табличные тесты.
- `stitch.rs` — фикстуры из `verbose_json` с искусственным overlap.
- `groq/types.rs` — десериализация эталонного `verbose_json`.
- `chunk.rs` — тесты на синтетическом `silencedetect`-выводе.
- `progress.rs` — throttle-тесты.

### 7.2 Integration

- Тестовый MP4 5 сек → весь pipeline с mock-сервером Groq (`wiremock`).
- Проверить 429 с `Retry-After`.
- Проверить timeout / connection reset.
- Проверить 401 без retry и с корректным событием на фронт.
- Проверить cancel в момент `transcribing` и очистку временных файлов.

### 7.3 E2E

- Запуск приложения.
- Drop одного файла.
- Проверка появления `.txt`.
- Можно использовать `tauri-driver` или отдельный smoke-тест.

### 7.4 Логи и метрики

- `tracing` + rolling daily log в `%APPDATA%\VideoTranscriber\logs\app.log`.
- На каждый stage — span с `job_id`, `stage`, `attempt`.
- `info!` на старт/конец, `warn!` на retry, `error!` на финальный fail.
- Опционально: in-app панель «Журнал».

---

## 8. Обновлённый план MVP

### День 1 — фундамент

- Tauri 2 init + Solid.js шаблон.
- `tauri-specta` → автогенерация типов.
- `tracing` + panic handler.
- `AppState`, инициализация SQLite, миграция `0001_init`.
- `FfmpegAdapter`: `probe` + `extract_audio` с прогрессом.
- Bundling ffmpeg sidecar и `cb.rnnn`.
- Команды: `enqueue_files`, `list_jobs`.

### День 2 — первый сквозной прогон

- `GroqClient::transcribe` с прогрессом аплоада.
- `JobScheduler` с двумя семафорами + rate-limiter.
- `Pipeline::run` без чанкинга и постобработки.
- Запись `.txt` атомарно.
- Keyring + страница Settings.

### День 3 — UX и устойчивость

- Drop-zone, виртуальный список, `JobRow` со всеми статусами.
- Throttled `queue:tick` события.
- Retry с exp-backoff.
- Классификация ошибок.
- Cancel + cleanup временных файлов.
- Inline-редактор транскрипта + autosave.

### День 4 — полировка перед релизом

- Settings полностью.
- Экспорт SRT/JSON из уже сохранённого `verbose_json`.
- Кэш по хешу файла.

### После MVP

- Fallback-чанкинг.
- Склейка чанков.
- Postprocess через Llama.
- Tauri Updater.
- Авто-распознавание языка.
- Журнал ошибок в UI.

---

## 9. Открытые вопросы перед стартом

1. **Tauri 1 или 2?** Рекомендация: Tauri 2.
2. **Куда писать `.txt`**, если исходная папка read-only? Нужна выходная папка как fallback.
3. **Что считать дубликатом** при повторном D&D того же файла? Рекомендация: игнорировать с тостом «уже в очереди».
4. **Поведение при выходе**: если очередь активна, спрашивать подтверждение. Опционально — сворачивание в трей.
5. **Формат экспорта по умолчанию**: `.txt`, остальные — по чекбоксам.
6. **Лимит размера очереди и временных файлов**: удалять `chunk_*.opus` сразу после успешной транскрибации чанка, полный `audio.opus` — после `Done`.

---

## Итог

Спецификация хорошая в части выбора технологий и pipeline, но не покрывает три критичных слоя:

1. Ресурсное планирование: CPU и network должны быть разными пулами.
2. Надёжность: отмена, очистка, классификация ошибок, атомарная запись.
3. Контракт между Rust и Solid.js: типы, события, throttle.

Предложенная архитектура:

- **Tauri** — тонкий слой маршаллинга и валидации.
- **Core** — оркестрация, две независимые семафор-очереди, token bucket на Groq, типизированная state machine.
- **Adapters** — FFmpeg, Groq, SQLite, Keyring.
- **UI** — Solid stores, виртуальный список, батчевые события, редактор с autosave.
- **Quality gates** — `tracing`, `secrecy`, `tauri-specta`, mock-сервер Groq для тестов.

Главный практический вывод: строить MVP нужно не вокруг «одного запроса к Whisper», а вокруг устойчивой очереди задач, потому что именно очередь, retry, отмена, временные файлы, прогресс и восстановление после перезапуска определят качество реального пользовательского опыта.
