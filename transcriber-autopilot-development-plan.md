# VideoTranscriber — план автопилотной разработки с AI

Дата подготовки: 2026-04-28

Документ предназначен для поэтапной разработки VideoTranscriber с помощью AI-автопилота. Он разбивает проект на крупные самостоятельные блоки, каждый из которых можно отдавать AI как отдельную большую задачу. Под каждым блоком есть готовый промпт, требования, зависимости и критерии готовности.

Основа документа:

- `transcriber-spec.md` — исходная техническая спецификация.
- `transcriber-architecture-analysis.md` — детальный архитектурный анализ и целевая архитектура.

Главная идея: AI-автопилот должен не просто писать отдельные функции, а строить цельную программу с устойчивой очередью, типизированным IPC, безопасным хранением ключей, прогрессом, retry, отменой, SQLite-историей и качественным UX.

---

## 0. Общая стратегия автопилотной разработки

### 0.1 Как использовать этот документ

1. Выполнять блоки преимущественно по порядку.
2. Перед каждым новым блоком давать AI текущий контекст проекта и соответствующий промпт из этого документа.
3. После каждого блока проверять сборку, запуск тестов и фактические изменения.
4. Не смешивать несколько крупных блоков в одном запуске AI, если они меняют одни и те же файлы.
5. Блоки `A`–`J` формируют полноценный MVP.
6. Блоки `K`–`N` — усиление после MVP.
7. Блок `U` — действия пользователя, которые AI не сможет полноценно выполнить без внешнего доступа, секретов или ручных решений.

### 0.2 Глобальные правила для всех AI-задач

Каждый AI-автопилот должен соблюдать следующие правила:

- Сначала изучить `transcriber-spec.md` и `transcriber-architecture-analysis.md`.
- Не хардкодить API-ключи, токены, личные пути пользователя и секреты.
- Не делать реальные платные запросы к Groq в тестах без явного разрешения пользователя.
- Все сетевые интеграции покрывать моками или абстракциями.
- Не запускать долгоживущие dev-серверы без необходимости.
- Все пути к пользовательским файлам передавать через безопасные API, без shell-строк.
- FFmpeg/ffprobe запускать только через аргументы процесса, не через конкатенацию командной строки.
- Все критичные операции должны возвращать понятные ошибки.
- При каждом блоке обновлять документацию, если меняется поведение или структура проекта.
- После реализации запускать доступные проверки: `cargo fmt`, `cargo clippy`, `cargo test`, `npm/pnpm/yarn lint`, `npm/pnpm/yarn test`, `npm/pnpm/yarn build` — в зависимости от того, что уже есть в проекте.

### 0.3 Общий Definition of Done для каждого блока

Блок считается завершённым, если:

- Код компилируется.
- Основные тесты проходят или явно описано, почему они не могут быть запущены.
- Нет хардкода секретов.
- Нет временных debug-заглушек, которые ломают дальнейшую разработку.
- Новые публичные API/команды/типы документированы.
- Ошибки возвращаются типизированно, а не через хаотичные строки.
- Изменения совместимы с целевой архитектурой из `transcriber-architecture-analysis.md`.

---

# MVP-блоки

## Блок A. Инициализация проекта, стек и базовая структура

### Цель

Создать рабочий каркас приложения на Tauri + Solid.js + Rust, который можно собрать, запустить и дальше расширять. Это фундамент для всего проекта.

### Зависимости

- Выполняется первым.
- Требует установленного Rust, Node.js и пакетного менеджера.
- Не требует Groq API key.
- Не требует ffmpeg-бинарей на этом этапе, но структура под них должна быть предусмотрена.

### Что должен сделать AI

- Инициализировать Tauri 2 проект с Solid.js + TypeScript.
- Настроить структуру каталогов, близкую к архитектуре:
  - `src-tauri/src/app`
  - `src-tauri/src/core`
  - `src-tauri/src/adapters`
  - `src-tauri/src/db`
  - `src-tauri/src/types`
  - `src-tauri/src/telemetry.rs`
  - frontend-слой с `src/stores`, `src/pages`, `src/components`, `src/ipc`.
- Настроить базовые npm/pnpm/yarn scripts.
- Настроить Rust workspace или обычный Tauri layout — выбрать самый практичный вариант для проекта.
- Добавить базовую главную страницу Solid.js.
- Добавить минимальную Tauri-команду `health_check`, чтобы проверить IPC.
- Добавить базовую страницу/экран приложения: заголовок, статус backend-соединения, заглушка очереди.
- Добавить `.gitignore`, если его нет.
- Добавить начальный `README.md` с командами разработки.

### Важные решения

- Рекомендация: использовать Tauri 2.
- Frontend: Solid.js + TypeScript + Vite.
- Стили: можно использовать CSS modules, Tailwind или UnoCSS. Предпочтительно выбрать один простой вариант и не усложнять.
- Не тянуть тяжёлые UI-фреймворки, если можно сделать аккуратный минималистичный интерфейс.

### Критерии готовности

- Приложение запускается в dev-режиме.
- Frontend может вызвать backend-команду `health_check`.
- Проект имеет понятную структуру для дальнейшего расширения.
- Команды сборки и проверки описаны в `README.md`.

### Промпт для AI-автопилота

> Ты работаешь в проекте `VideoTranscriber`. Сначала изучи `transcriber-spec.md` и `transcriber-architecture-analysis.md`. Нужно создать фундамент приложения на Tauri 2 + Solid.js + TypeScript + Rust. Реализуй рабочий каркас, который можно запустить и расширять дальше.
>
> Требования: создай структуру каталогов под backend (`app`, `core`, `adapters`, `db`, `types`, `telemetry`) и frontend (`stores`, `pages`, `components`, `ipc`). Добавь минимальную Tauri-команду `health_check` и вызови её из Solid UI. Сделай стартовую страницу с заголовком VideoTranscriber, статусом backend-соединения и пустым блоком очереди. Настрой scripts для dev/build/check. Добавь README с командами запуска.
>
> Не реализуй пока Groq, SQLite, ffmpeg и очередь — только качественный каркас. Не хардкодь секреты. После изменений запусти доступные formatter/build/check команды и кратко отчитайся, что сделано и что осталось.

---

## Блок B. Типизированный домен, IPC-контракт и события

### Цель

Заложить единый контракт между Rust backend и Solid frontend: типы задач, состояния, настройки, ошибки, команды и события. Это предотвращает хаос на стыке UI и backend.

### Зависимости

- Требует завершённого блока A.
- Не требует Groq API key.
- Не требует SQLite и ffmpeg.

### Что должен сделать AI

- Создать доменные Rust-типы:
  - `JobId`
  - `Job`
  - `JobState`
  - `JobSettings`
  - `Settings`
  - `ExportFormat`
  - `OutputLocation`
  - `ConflictPolicy`
  - `Transcript`
  - `TranscriptSegment`
  - `AppError`
  - `AppErrorView`
- Настроить типизированную генерацию bindings для frontend через `specta`, `tauri-specta` или другой совместимый подход.
- Если автогенерация bindings слишком затратна на этом этапе — создать аккуратный ручной TypeScript contract, но явно оставить задачу на замену автогенерацией.
- Добавить Tauri-команды-заглушки:
  - `enqueue_files`
  - `cancel_job`
  - `retry_job`
  - `pause_queue`
  - `resume_queue`
  - `list_jobs`
  - `get_transcript`
  - `save_transcript_edit`
  - `export_transcript`
  - `get_settings`
  - `set_settings`
  - `save_api_key`
- Добавить frontend IPC wrappers.
- Добавить базовый event contract:
  - `queue:tick`
  - `job:done`
  - `job:failed`
  - `job:cancelled`
  - `queue:idle`
  - `app:error`
  - `app:rate-limited`
  - `app:auth-failed`

### Важные решения

- Ошибки backend должны сериализоваться в UI-friendly формат.
- `JobState` должен поддерживать прогресс для `Extracting`, `Chunking`, `Uploading`, а также indeterminate-состояния для `Transcribing`, `Stitching`, `Postprocessing`.
- Все настройки, влияющие на результат транскрибации, должны попадать в `JobSettings` snapshot.

### Критерии готовности

- Frontend использует типы или wrappers, а не произвольные строки.
- Команды доступны, пусть пока и возвращают заглушки.
- Состояния задач представлены единообразно.
- Ошибки backend приводятся к `AppErrorView`.

### Промпт для AI-автопилота

> Продолжи проект VideoTranscriber после базовой инициализации. Изучи `transcriber-spec.md` и `transcriber-architecture-analysis.md`. Нужно заложить типизированный контракт между Rust backend и Solid frontend.
>
> Реализуй доменные типы задач, состояний, настроек, транскриптов и ошибок. Добавь Tauri-команды-заглушки для всей будущей функциональности: enqueue/list/cancel/retry/pause/resume/export/settings/api-key/transcript. Настрой генерацию TypeScript bindings через `specta`/`tauri-specta`, если это практически совместимо с текущей версией Tauri. Если нет — сделай ручной `src/ipc/types.ts` с явным TODO на автогенерацию.
>
> Добавь frontend IPC wrappers и общий event contract для `queue:tick`, `job:done`, `job:failed`, `job:cancelled`, `queue:idle`, `app:error`, `app:rate-limited`, `app:auth-failed`. Команды пока могут возвращать тестовые данные, но их сигнатуры должны быть максимально близки к финальным.
>
> Не реализуй пока реальную очередь, Groq, SQLite или ffmpeg. После изменений запусти formatter/build/typecheck и опиши результат.

---

## Блок C. SQLite, настройки, keyring и слой репозиториев

### Цель

Добавить долговременное хранение истории задач, транскриптов, кэша и настроек. API-ключ Groq должен храниться безопасно через OS keychain.

### Зависимости

- Требует блока B.
- Не требует Groq API key для реализации, но для ручной проверки пользователь позже введёт ключ.
- Не требует ffmpeg.

### Что должен сделать AI

- Подключить SQLite через `sqlx` или другой зрелый Rust crate.
- Создать миграции:
  - `jobs`
  - `transcripts`
  - `cache`
  - `settings`, если настройки решено хранить в SQLite.
- Реализовать init базы в `%APPDATA%`/app data directory Tauri.
- Реализовать repositories:
  - `JobRepo`
  - `TranscriptRepo`
  - `CacheRepo`
  - `SettingsRepo`
- Реализовать сохранение и чтение настроек.
- Реализовать keyring-adapter для Groq API key:
  - `save_api_key`
  - `has_api_key`
  - `delete_api_key`
  - внутреннее чтение ключа для Groq-клиента позже.
- Обновить Tauri-команды settings/api-key, чтобы они работали реально.
- Добавить unit/integration tests для repositories там, где возможно.

### Важные решения

- API-ключ не хранить в SQLite.
- В логах не показывать API-ключ.
- `settings_snapshot` должен сохраняться в `jobs`, чтобы повторные задачи были воспроизводимыми.
- Транскрипты хранить отдельно от jobs, чтобы список задач не таскал большие тексты.

### Критерии готовности

- При запуске приложения база создаётся автоматически.
- Настройки сохраняются между перезапусками.
- API-ключ сохраняется в keyring, а не в БД.
- `list_jobs` может читать задачи из БД.
- Миграции применяются стабильно.

### Промпт для AI-автопилота

> Продолжи проект VideoTranscriber. Нужно реализовать persistence-слой: SQLite, настройки, репозитории и безопасное хранение Groq API key через OS keychain. Сначала изучи `transcriber-spec.md` и `transcriber-architecture-analysis.md`, затем текущий код.
>
> Подключи SQLite, создай миграции `jobs`, `transcripts`, `cache` и при необходимости `settings`. База должна жить в app data directory приложения, а не в корне проекта. Реализуй `JobRepo`, `TranscriptRepo`, `CacheRepo`, `SettingsRepo`. Настройки должны сохраняться и читаться через Tauri-команды. API-ключ Groq должен храниться только через keyring/Windows Credential Manager, не в SQLite и не в логах.
>
> Обнови команды `get_settings`, `set_settings`, `save_api_key`, `list_jobs`. Добавь тесты для репозиториев, если инфраструктура позволяет. Не реализуй пока ffmpeg, Groq-запросы и реальную очередь обработки. После изменений запусти formatter/build/tests и отчитайся.

---

## Блок D. FFmpeg/ffprobe adapter, валидация медиа и извлечение аудио

### Цель

Реализовать локальную подготовку медиа: проверка MP4, извлечение аудио в Opus mono 16kHz 32kbps, шумоподавление, прогресс, временные файлы и cleanup.

### Зависимости

- Требует блоков B и C.
- Желательно, чтобы пользователь заранее положил ffmpeg/ffprobe sidecar binaries и rnnoise model, но AI может подготовить структуру и graceful errors.
- Не требует Groq API key.

### Что должен сделать AI

- Реализовать `FfmpegAdapter`:
  - поиск sidecar ffmpeg/ffprobe или конфигурируемый путь для dev-режима;
  - `probe(path) -> ProbeResult`;
  - проверка наличия аудиодорожки;
  - получение длительности, кодеков, размера;
  - `extract_audio(input, output, cancel, progress)`;
  - прогресс через `-progress pipe:2` и `out_time_us`;
  - безопасный запуск процесса без shell-конкатенации;
  - отмена процесса через `CancellationToken`;
  - cleanup временных файлов.
- Команда извлечения:
  - mono `-ac 1`;
  - 16kHz `-ar 16000`;
  - Opus `-c:a libopus -b:a 32k`;
  - без видео `-vn`;
  - шумоподавление через `arnndn`, если model file доступен;
  - fallback без `arnndn`, если модель не найдена, но с предупреждением.
- Добавить тесты для парсинга ffmpeg progress и ffprobe JSON на фикстурах.
- Добавить dev-команду или internal test command для проверки `probe` и `extract` на локальном файле, если это удобно.

### Важные решения

- Не использовать shell-строки.
- Не ломать pipeline, если rnnoise model отсутствует: лучше warning + extraction без фильтра.
- Временные файлы хранить в app cache/temp directory.
- После успешной обработки временные файлы удалять.
- На Windows учитывать пробелы и кириллицу в путях.

### Критерии готовности

- Можно проверить MP4 через `ffprobe`.
- MP4 без аудиодорожки даёт понятную ошибку.
- Аудио извлекается в `.opus`.
- UI/backend получает прогресс извлечения.
- Отмена корректно убивает ffmpeg и чистит temp.

### Промпт для AI-автопилота

> Реализуй FFmpeg/ffprobe слой для VideoTranscriber. Изучи текущую архитектуру, `transcriber-spec.md` и `transcriber-architecture-analysis.md`. Нужен безопасный `FfmpegAdapter` для probe и extract.
>
> Реализуй `probe`: через ffprobe JSON определить длительность, наличие аудио, streams, format, размер. Реализуй `extract_audio`: конвертация входного MP4 в Opus mono 16kHz 32kbps без видео, с шумоподавлением `arnndn`, если доступна rnnoise model. Прогресс считывай через `-progress pipe:2`, не через fragile human-readable stderr. Запуск только через argv, без shell-конкатенации. Поддержи cancellation token: при отмене убить процесс и удалить временные файлы.
>
> Если ffmpeg/ffprobe binaries или rnnoise model не найдены, верни понятную ошибку или warning с fallback без шумоподавления. Добавь тесты для парсинга ffprobe/progress на фикстурах. Не реализуй пока Groq и полную очередь. После изменений запусти formatter/build/tests и отчитайся.

---

## Блок E. Groq API client, upload progress, retry и rate limiting

### Цель

Реализовать сетевой слой для Groq Whisper и подготовить опциональный Llama postprocess. При этом нельзя жёстко завязать тесты на реальный API.

### Зависимости

- Требует блоков B, C, D.
- Для live-проверки нужен Groq API key от пользователя, но реализация и тесты должны работать без него.

### Что должен сделать AI

- Реализовать `GroqClient`:
  - base URL `https://api.groq.com/openai/v1`;
  - чтение API key через keyring adapter;
  - `transcribe(audio_path, opts, on_upload, cancel) -> VerboseJson`;
  - multipart upload с реальным progress callback;
  - `response_format=verbose_json`;
  - `model=whisper-large-v3-turbo`;
  - `language=ru` по умолчанию;
  - `temperature=0`;
  - prompt из настроек.
- Реализовать типы ответа `VerboseJson`, `Segment`, `Word` если доступно.
- Реализовать классификацию HTTP-ошибок:
  - 401 auth;
  - 429 rate limited с `Retry-After`;
  - 5xx transient;
  - timeouts/network transient;
  - 4xx validation/non-retryable.
- Реализовать retry/backoff модуль.
- Реализовать rate limiter под 30 rpm, совместимый с `net_sem`.
- Реализовать mock tests через `wiremock`, `httpmock` или аналог.
- Подготовить, но не обязательно полностью включать, `postprocess_transcript` через Groq Llama.

### Важные решения

- Не делать реальные Groq-вызовы в обычных тестах.
- Upload progress должен работать для UI через callback.
- Cancellation должен обрывать запрос.
- API key не логировать.
- Логи могут показывать request id/status/duration, но не payload с транскриптом.

### Критерии готовности

- Groq client может быть протестирован мок-сервером.
- 429/5xx/timeout ретраятся корректно.
- 401 не ретраится и классифицируется как auth error.
- Upload progress отдаёт байты uploaded/total.
- Типы verbose_json десериализуются.

### Промпт для AI-автопилота

> Реализуй сетевой слой Groq для VideoTranscriber. Изучи текущий проект, `transcriber-spec.md` и `transcriber-architecture-analysis.md`. Нужен production-ready `GroqClient` для Whisper Large v3 Turbo с прогрессом upload, cancellation, retry/backoff и rate limiting.
>
> Реализуй multipart upload аудиофайла в `/openai/v1/audio/transcriptions` с параметрами: model `whisper-large-v3-turbo`, language `ru`, response_format `verbose_json`, temperature `0`, prompt из настроек. API key бери только из keyring adapter, не хардкодь и не логируй. Реализуй deserialization verbose_json. Реализуй классификацию ошибок: 401 auth без retry, 429 с Retry-After, 5xx/network/timeout как transient, прочие 4xx как non-retryable. Добавь retry/backoff с jitter и rate limiter под 30 rpm.
>
> Добавь mock tests через локальный mock HTTP server. Обычные тесты не должны ходить в настоящий Groq. Если есть возможность, добавь отдельный ignored/manual тест для live-проверки при наличии API key. Не реализуй пока полную очередь UI, но подготовь API для pipeline. После изменений запусти formatter/build/tests и отчитайся.

---

## Блок F. Core queue scheduler и полный pipeline без чанкинга

### Цель

Собрать первый настоящий end-to-end pipeline для обычных файлов: `queued -> probing -> extracting -> uploading -> transcribing -> writing -> done/error`. Без fallback-чанкинга, но с retry, cancellation, прогрессом, SQLite и событиями.

### Зависимости

- Требует блоков B, C, D, E.
- Для полного live-прогона нужен Groq API key.
- Без API key pipeline должен корректно падать в понятную auth/config error.

### Что должен сделать AI

- Реализовать `JobScheduler`:
  - очередь задач;
  - `cpu_sem` для ffmpeg;
  - `net_sem` для Groq;
  - rate limiter;
  - cancellation tokens;
  - retry command;
  - pause/resume queue.
- Реализовать `PipelineCtx`.
- Реализовать `pipeline::run` без чанкинга:
  - создать job в БД;
  - `Probing`;
  - `Extracting` с progress;
  - `Uploading` с progress;
  - `Transcribing` indeterminate;
  - сохранить transcript в БД;
  - записать `.txt` рядом с видео или в output folder;
  - `Done`;
  - при ошибке `Failed` с readable error.
- Реализовать event bus/throttling:
  - `queue:tick` не чаще примерно 10 Hz;
  - редкие one-shot events.
- Обновить Tauri-команды:
  - `enqueue_files` теперь реально ставит задачи;
  - `list_jobs` читает актуальные задачи;
  - `cancel_job` отменяет;
  - `retry_job` перезапускает failed/cancelled;
  - `pause_queue`/`resume_queue` работают.
- Реализовать cleanup temp files.
- Добавить integration tests с mock Groq и fake/small media там, где реально.

### Важные решения

- Не запускать много ffmpeg одновременно на слабом ноутбуке.
- Не терять состояние при перезапуске: активные задачи после рестарта должны стать `Failed`, `Cancelled` или `Queued` по выбранной политике.
- Запись результата должна быть атомарной.
- Ошибки должны быть полезны пользователю.

### Критерии готовности

- Можно поставить файл в очередь.
- Состояния меняются по pipeline.
- Прогресс extraction/upload доходит до UI event bus.
- Текст сохраняется в `.txt`.
- Ошибка auth/нет ffmpeg/нет аудио отображается корректно.
- Cancel действительно останавливает активную задачу.

### Промпт для AI-автопилота

> Собери core scheduler и первый end-to-end pipeline VideoTranscriber без fallback-чанкинга. Изучи текущий код и документы. Нужно, чтобы приложение реально могло принять MP4, проверить его, извлечь Opus, отправить в Groq, сохранить transcript и записать `.txt`.
>
> Реализуй `JobScheduler` с `cpu_sem` и `net_sem`, cancellation tokens, pause/resume, retry, rate limiter и throttled event bus. Реализуй `PipelineCtx` и `pipeline::run`: Probing -> Extracting -> Uploading -> Transcribing -> Writing -> Done или Failed. Все переходы состояния сохраняй в SQLite и отправляй во frontend через `queue:tick`. Временные файлы чисти. `.txt` записывай атомарно с учётом conflict policy.
>
> Обнови Tauri-команды `enqueue_files`, `list_jobs`, `cancel_job`, `retry_job`, `pause_queue`, `resume_queue`. Без API key pipeline должен давать понятную ошибку, а не panic. Для тестов используй mock Groq, не настоящий API. После изменений запусти formatter/build/tests и отчитайся.

---

## Блок G. Result writer, экспорт TXT/SRT/JSON и редактируемые транскрипты

### Цель

Сделать качественную работу с результатами: сохранение `.txt`, экспорт `.srt`/`.json`, хранение raw/edited transcript, атомарная запись, conflict policy.

### Зависимости

- Требует блоков B, C, F.
- Не требует live Groq, можно использовать сохранённые/mocked transcripts.

### Что должен сделать AI

- Реализовать `write_result`:
  - TXT;
  - SRT;
  - JSON/verbose_json;
  - атомарная запись через temp + rename;
  - conflict policy: overwrite/suffix/skip.
- Реализовать форматирование SRT:
  - корректные `HH:MM:SS,mmm`;
  - нумерация сегментов;
  - переносы строк по разумной длине.
- Реализовать хранение `plain_text`, `segments_json`, `edited_text`.
- Обновить команды:
  - `get_transcript`;
  - `save_transcript_edit`;
  - `export_transcript`.
- Если пользователь редактировал transcript, TXT-export должен использовать `edited_text`, если это выбранное поведение.
- Добавить unit tests для SRT formatter, atomic write и conflict policy.

### Важные решения

- Raw transcript и edited transcript не смешивать без явного правила.
- JSON export должен сохранять достаточно данных для повторного SRT-export без повторного Groq-запроса.
- Не перезаписывать существующий пользовательский файл неожиданно.

### Критерии готовности

- Можно экспортировать TXT/SRT/JSON из результата.
- Edited transcript сохраняется между перезапусками.
- SRT корректно открывается в стандартных плеерах.
- Conflict policy работает.

### Промпт для AI-автопилота

> Реализуй слой результатов и экспорта для VideoTranscriber. Изучи текущий код и документы. Нужно качественно сохранять и экспортировать транскрипты в TXT, SRT и JSON, а также поддержать ручные правки текста.
>
> Реализуй атомарную запись файлов с conflict policy overwrite/suffix/skip. Добавь SRT formatter с корректными таймкодами `HH:MM:SS,mmm`, нумерацией и разумными переносами строк. JSON export должен сохранять raw verbose_json/segments. Команды `get_transcript`, `save_transcript_edit`, `export_transcript` должны работать реально. Если есть `edited_text`, продумай и реализуй правило, когда он используется для TXT export.
>
> Добавь unit tests для SRT formatter, atomic write и conflict policy. Не делай повторных Groq-запросов для экспорта. После изменений запусти formatter/build/tests и отчитайся.

---

## Блок H. Основной UI: очередь, drag & drop, прогресс и управление задачами

### Цель

Сделать пользовательский интерфейс очереди: drag & drop файлов/папок, статусы, прогресс, ошибки, retry/cancel, pause/resume.

### Зависимости

- Требует блоков B и F.
- Для красивого отображения результатов полезен блок G, но можно делать параллельно, если не трогать одни файлы.

### Что должен сделать AI

- Реализовать главный экран очереди:
  - drag & drop зона;
  - кнопка выбора файлов;
  - список задач;
  - статус каждой задачи;
  - progress bar для extracting/uploading;
  - indeterminate state для transcribing;
  - кнопки cancel/retry/open/export;
  - pause/resume queue;
  - фильтры all/active/failed/done.
- Подключить `queue:tick` events.
- Реализовать frontend store:
  - map задач по id;
  - order задач;
  - точечные обновления.
- Добавить виртуализацию списка, если задач много, либо подготовить структуру под неё.
- Реализовать toasts/notifications для ошибок и завершения.
- Обработать drop папок: если Tauri/OS отдаёт папку, отправить backend на рекурсивный scan или корректно показать, что папки пока не поддерживаются. Лучше поддержать через backend walk.
- UX должен быть понятным для пользователя без CLI.

### Важные решения

- Не ререндерить весь список на каждый progress tick.
- Event updates должны применяться идемпотентно.
- Ошибка должна показывать короткий текст и раскрываемые детали.
- Не блокировать UI во время обработки.

### Критерии готовности

- Пользователь может добавить несколько файлов через D&D.
- Очередь показывает реальные состояния.
- Можно отменить и повторить задачу.
- Ошибки видны и понятны.
- UI остаётся отзывчивым при десятках задач.

### Промпт для AI-автопилота

> Реализуй основной UI очереди VideoTranscriber на Solid.js. Изучи документы и текущий frontend/backend контракт. Нужно сделать удобный интерфейс для drag & drop, отображения задач, прогресса, ошибок и управления очередью.
>
> Создай главный экран очереди: drop-zone, выбор файлов, список задач, прогресс для extracting/uploading, indeterminate для transcribing, статусы done/failed/cancelled, кнопки cancel/retry/open/export, pause/resume, фильтры all/active/failed/done. Подключи `queue:tick` events и обновляй store точечно по id, не перерисовывая всё без необходимости. Добавь toasts для ошибок/успеха. Если возможно, поддержи drop папок через backend scan; если нет — сделай понятную ошибку и TODO.
>
> UI должен быть минималистичным, но пригодным для реального использования. Не хардкодь фиктивные задачи, кроме dev fallback, если backend недоступен. После изменений запусти frontend typecheck/build/lint и общую сборку, если возможно.

---

## Блок I. Settings UI, API key UX и экран транскрипта/редактирования

### Цель

Дать пользователю управление настройками и удобную работу с готовым транскриптом: просмотр, редактирование, автосохранение, экспорт.

### Зависимости

- Требует блоков C, G, H.
- Не требует live Groq, кроме опциональной проверки API key.

### Что должен сделать AI

- Реализовать страницу Settings:
  - API key field с маской;
  - save/delete key;
  - индикатор наличия ключа;
  - язык по умолчанию `ru`;
  - prompt textarea;
  - output formats: txt/srt/json;
  - concurrency settings: CPU/network;
  - conflict policy;
  - output location;
  - postprocess toggle, если типы уже есть.
- Реализовать экран Detail/Transcript:
  - отображение текста;
  - textarea/editor;
  - autosave с debounce;
  - segments/timestamps list;
  - copy to clipboard;
  - export buttons;
  - open output file/folder, если Tauri permissions позволяют.
- Добавить UX для auth error: при `app:auth-failed` предлагать открыть Settings.
- Настройки должны сохраняться и применяться к новым задачам.

### Важные решения

- Не показывать API key полностью после сохранения.
- Prompt должен быть редактируемым и иметь sensible default.
- Изменение настроек не должно ломать уже запущенные задачи: job должен иметь settings snapshot.
- Autosave должен быть debounce, не запись на каждый символ.

### Критерии готовности

- Пользователь может ввести и сохранить API key.
- Пользователь может настроить язык/prompt/formats/concurrency/conflict policy.
- Можно открыть готовый transcript, отредактировать и экспортировать.
- Auth error ведёт пользователя в настройки.

### Промпт для AI-автопилота

> Реализуй Settings UI и экран просмотра/редактирования транскрипта для VideoTranscriber. Изучи документы и текущий код. Нужно дать пользователю полноценное управление настройками и результатами.
>
> Settings: API key с маской, save/delete, индикатор наличия ключа, язык `ru`, prompt textarea с дефолтом, output formats txt/srt/json, concurrency CPU/network, conflict policy, output location, postprocess toggle если поддерживается. API key не показывай полностью и не храни во frontend state дольше необходимого.
>
> Detail screen: отображение transcript, editor/textarea с debounce autosave, список segments с таймкодами, copy to clipboard, export buttons, open file/folder если разрешено. При `app:auth-failed` показывай toast/dialog с переходом в Settings. Настройки должны сохраняться через backend и применяться только к новым задачам через settings snapshot.
>
> После изменений запусти frontend typecheck/build/lint и общие проверки.

---

## Блок J. Логи, ошибки, безопасность, polish и MVP hardening

### Цель

Превратить работающий прототип в надёжный MVP: логирование, обработка panic, понятные ошибки, безопасность Tauri permissions, CSP, UX-polish, документация.

### Зависимости

- Требует блоков A–I.

### Что должен сделать AI

- Настроить `tracing`:
  - console в dev;
  - rolling file logs в app data logs directory;
  - spans по `job_id`, `stage`, `attempt`.
- Добавить panic hook:
  - логировать panic;
  - показывать пользователю понятное сообщение.
- Привести error mapping к единому виду:
  - validation;
  - local IO;
  - auth;
  - rate limit;
  - network;
  - server;
  - cancelled;
  - unknown.
- Проверить, что secrets не логируются.
- Настроить Tauri permissions/capabilities минимально необходимым образом.
- Настроить CSP.
- Добавить in-app журнал или кнопку открытия папки логов.
- Добавить graceful startup recovery:
  - задачи, оставшиеся `Extracting/Uploading/Transcribing` после аварийного выхода, переводить в recoverable state.
- Обновить README:
  - установка зависимостей;
  - ffmpeg sidecar;
  - Groq API key;
  - dev mode;
  - build;
  - troubleshooting.

### Важные решения

- Логи не должны содержать transcript text, API key и приватные данные больше необходимого.
- Ошибки должны помогать пользователю действовать: «добавьте API key», «нет ffmpeg», «нет аудиодорожки», «rate limit, повторяем через N сек».
- MVP должен быть безопасен для реального локального использования.

### Критерии готовности

- Логи пишутся в правильное место.
- Panic не приводит к молчаливому падению без информации.
- Ошибки в UI понятны.
- Tauri permissions не избыточны.
- README позволяет новому разработчику запустить проект.

### Промпт для AI-автопилота

> Проведи MVP hardening VideoTranscriber. Изучи проект и документы. Нужно улучшить логи, ошибки, безопасность, startup recovery и документацию, не ломая реализованную функциональность.
>
> Настрой `tracing` с console dev и rolling file logs в app data. Добавь spans с `job_id`, `stage`, `attempt`. Добавь panic hook. Проверь error taxonomy и mapping в UI-friendly ошибки. Убедись, что API key и transcript content не логируются. Настрой Tauri capabilities/permissions и CSP минимально необходимым образом. Добавь in-app доступ к логам или кнопку открытия папки логов. Реализуй recovery активных задач после аварийного выхода. Обнови README: установка, ffmpeg sidecar, Groq API key, dev/build, troubleshooting.
>
> После изменений запусти formatter, clippy, tests, frontend checks и сборку. Исправь найденные проблемы, не упрощая архитектуру.

---

# Post-MVP блоки

## Блок K. Кэш и дедупликация задач

### Цель

Ускорить повторную обработку файлов и защититься от случайных дублей в очереди.

### Зависимости

- Требует блоков C, F, G, H.

### Что должен сделать AI

- Реализовать BLAKE3 content hash.
- Реализовать settings fingerprint.
- Реализовать cache key: `file_hash + settings_fingerprint`.
- Добавить weak-key для дедупликации текущего батча: размер, mtime, hash первых мегабайт.
- При enqueue определять очевидные дубли в текущей очереди.
- Перед обработкой проверять cache.
- При cache hit:
  - не отправлять в Groq;
  - переиспользовать transcript;
  - при необходимости записать новый output file рядом с текущим видео.
- Добавить UI-индикатор `Skipped/Cached` или `Done from cache`.
- Добавить tests для cache key и dedup.

### Важные решения

- Полный hash больших файлов считать аккуратно, не блокируя UI.
- Изменение prompt/language/model должно инвалидировать кэш.
- Cache hit не должен случайно возвращать результат с другими настройками.

### Критерии готовности

- Повторная обработка того же файла с теми же настройками не вызывает Groq.
- Дубликаты в drag/drop партии не создают лишних задач.
- Изменение настроек вызывает новую обработку.

### Промпт для AI-автопилота

> Реализуй кэш и дедупликацию для VideoTranscriber. Изучи текущий pipeline, SQLite schema и документы. Нужно, чтобы повторная обработка того же файла с теми же настройками переиспользовала результат, а случайные дубли в очереди не создавали лишнюю работу.
>
> Добавь BLAKE3 full content hash, settings fingerprint и cache key `file_hash + settings_fingerprint`. Добавь weak-key для дедупликации текущего batch: size, mtime, hash первых мегабайт. При enqueue выявляй очевидные дубли. Перед Groq stage проверяй cache, при hit переиспользуй transcript и записывай output без повторного API call. Добавь UI-статус cached/skipped. Добавь tests для cache key, invalidation и dedup.
>
> Не блокируй UI на хешировании больших файлов. Не допускай cache hit при изменившихся prompt/language/model. После изменений запусти проверки и отчитайся.

---

## Блок L. Fallback-чанкинг, silencedetect и склейка чанков

### Цель

Поддержать аудиофайлы, которые после конвертации всё равно превышают лимит Groq 100 MB, сохранив качество и минимизировав дубли/пропуски на стыках.

### Зависимости

- Требует блоков D, E, F, G.
- Лучше выполнять после стабильного MVP.

### Что должен сделать AI

- Реализовать `chunk.rs`:
  - `silencedetect` через ffmpeg;
  - парсинг silence_start/silence_end;
  - выбор точек резки по тишине;
  - max chunk size около 80 MB;
  - overlap 5–10 секунд;
  - metadata `start_global`, `end_global`, `overlap_pre`, `overlap_post`.
- Реализовать нарезку аудио на `.opus` chunks.
- Изменить pipeline:
  - если opus > 100 MB, включить chunking;
  - отправлять chunks в Groq параллельно с учётом `net_sem` и rate limiter;
  - собирать results в исходном порядке.
- Реализовать `stitch.rs`:
  - сдвиг локальных таймкодов в глобальные;
  - дедупликация overlap по timestamp + text similarity;
  - нормализация токенов;
  - tests с искусственными overlapping segments.
- Обновить UI progress: chunk index/total.

### Важные решения

- Чанкинг — fallback, не default.
- Нужно резать в тишине, а не тупо каждые N минут.
- Overlap нужен, чтобы не терять слова на границах.
- Дедупликация должна быть тестируемой.

### Критерии готовности

- Большой opus >100 MB режется на chunks.
- Chunks отправляются параллельно, но не нарушают rate limit.
- Финальный transcript не содержит очевидных дублей на overlap.
- SRT таймкоды остаются глобальными и корректными.

### Промпт для AI-автопилота

> Реализуй fallback-чанкинг и склейку чанков для VideoTranscriber. Изучи документы, текущий ffmpeg adapter, Groq client и pipeline. Чанкинг должен включаться только если Opus после extraction больше лимита Groq 100 MB.
>
> Реализуй `silencedetect`, парсинг silence intervals, выбор границ резки по тишине, max chunk size около 80 MB, overlap 5–10 секунд. Нарезай `.opus` chunks и храни metadata start/end/overlap. Обнови pipeline: chunks отправляются в Groq параллельно с учётом `net_sem` и rate limiter, затем собираются по порядку.
>
> Реализуй stitch: локальные таймкоды -> глобальные, дедупликация overlap по timestamp + text similarity на нормализованных токенах. Добавь unit tests на synthetic overlapping segments и tests для parser silencedetect. Обнови UI progress для chunk index/total. После изменений запусти проверки и отчитайся.

---

## Блок M. Постобработка через Groq Llama

### Цель

Добавить опциональную чистку пунктуации и лёгкое улучшение читаемости транскрипта через Groq Llama без изменения смысла.

### Зависимости

- Требует блоков E, F, G, I.

### Что должен сделать AI

- Реализовать `GroqClient::postprocess_transcript`.
- Модель по умолчанию: `llama-3.1-8b-instant` или актуальная совместимая модель из настроек.
- Prompt должен запрещать:
  - менять смысл;
  - добавлять факты;
  - удалять важные слова;
  - переводить текст;
  - менять стиль сильнее необходимого.
- Поддержать настройку `postprocess_enabled`.
- Сохранять raw transcript и postprocessed transcript отдельно.
- UI должен показывать, что текст был постобработан.
- При ошибке postprocess не терять raw transcript.
- Добавить tests на prompt construction и mock Groq response.

### Важные решения

- Постобработка опциональна и выключена по умолчанию или явно отмечена.
- Нельзя портить timestamps: SRT должен строиться по raw segments, даже если plain text улучшен.
- Если Llama ошиблась, пользователь должен иметь возможность вернуться к raw.

### Критерии готовности

- Пользователь может включить postprocess.
- Raw transcript сохраняется всегда.
- Postprocess failure не ломает основную транскрибацию.
- UI показывает различие raw/processed/edited.

### Промпт для AI-автопилота

> Добавь опциональную постобработку транскрипта через Groq Llama. Изучи текущий Groq client, pipeline, settings и transcript storage. Нужно улучшать пунктуацию/читаемость, но не менять смысл.
>
> Реализуй `postprocess_transcript` через Groq chat/completions с моделью из настроек, дефолт `llama-3.1-8b-instant` или совместимый. Prompt должен строго запрещать изменение смысла, добавление фактов, перевод, удаление важных слов. Добавь настройку `postprocess_enabled`. Raw transcript сохраняй всегда, processed transcript сохраняй отдельно. Если postprocess падает, основная задача должна завершиться с raw transcript и warning, а не failed.
>
> Обнови UI: показать, что текст postprocessed, дать возможность видеть raw/processed/edited. Добавь mock tests для API и tests prompt construction. После изменений запусти проверки.

---

## Блок N. Packaging, updater, release pipeline и Windows-полировка

### Цель

Подготовить приложение к установке и обновлению на Windows: bundling, sidecars, updater, release artifacts, smoke-test.

### Зависимости

- Требует MVP блоков A–J.
- Для полного выполнения нужны пользовательские решения по signing/updater/GitHub.

### Что должен сделать AI

- Настроить Tauri bundle для Windows.
- Проверить включение sidecar binaries:
  - ffmpeg;
  - ffprobe;
  - rnnoise model.
- Настроить app metadata:
  - product name;
  - identifier;
  - version;
  - icons, если предоставлены.
- Подготовить GitHub Actions workflow:
  - install deps;
  - cargo checks;
  - frontend build;
  - tauri build;
  - upload artifacts.
- Подготовить Tauri updater config с placeholder-ключами или инструкциями.
- Добавить smoke checklist для релиза.
- Добавить troubleshooting для Windows Defender/SmartScreen/ffmpeg sidecar.

### Важные решения

- Code signing может потребовать ручных сертификатов.
- Updater требует signing key и hosting release manifest.
- Не публиковать секреты в репозитории.

### Критерии готовности

- Можно собрать Windows installer/bundle.
- CI хотя бы собирает проект.
- Sidecars попадают в bundle.
- README/release docs объясняют ручные шаги.

### Промпт для AI-автопилота

> Подготовь packaging и release pipeline для VideoTranscriber на Windows. Изучи текущий проект и документы. Нужно, чтобы приложение можно было собрать в установщик/bundle, sidecars попали внутрь, а CI проверял сборку.
>
> Настрой Tauri bundle metadata, Windows target, product name, identifier, version. Проверь конфигурацию sidecar binaries для ffmpeg/ffprobe и rnnoise model. Подготовь GitHub Actions workflow: install deps, frontend build, cargo fmt/clippy/test, tauri build, upload artifacts. Подготовь updater config с placeholder/instructions, но не добавляй секреты. Обнови README/release docs: build, signing, updater, Windows troubleshooting.
>
> Если code signing/updater keys требуют ручных действий пользователя, явно вынеси это в TODO/USER ACTIONS. После изменений запусти локальные проверки, насколько возможно.

---

# Блок U. Действия пользователя, которые нельзя полностью автоматизировать

Этот блок содержит задачи, которые AI-автопилот не сможет безопасно или полноценно выполнить самостоятельно, потому что нужны секреты, ручные решения, внешние аккаунты, лицензии, бинарные файлы или проверка на реальном окружении.

## U1. Установка локального окружения

Пользователь должен установить и проверить:

- Rust stable toolchain.
- Node.js LTS.
- Выбранный package manager: npm, pnpm или yarn.
- WebView2 Runtime на Windows, если отсутствует.
- Visual Studio Build Tools / Windows SDK, если Tauri/Rust сборка требует.

Проверка:

- `rustc --version`
- `cargo --version`
- `node --version`
- package manager version

## U2. Groq API key

Пользователь должен:

- Создать аккаунт Groq.
- Получить API key.
- Ввести его в Settings приложения.
- Не отправлять ключ в чат AI и не коммитить его в репозиторий.

AI может реализовать хранение ключа, но не должен знать сам ключ.

## U3. FFmpeg/ffprobe binaries

Пользователь должен принять решение по поставке ffmpeg:

- Скачать Windows static build ffmpeg/ffprobe из доверенного источника.
- Проверить лицензионные условия.
- Положить binaries в ожидаемую папку проекта, если packaging ещё не автоматизирован.
- Проверить, что ffmpeg собран с `libopus` и фильтром `arnndn`.

Минимальная проверка:

- `ffmpeg -filters` должен содержать `arnndn`.
- `ffmpeg -codecs` или `ffmpeg -encoders` должен содержать Opus/libopus.
- `ffprobe` должен запускаться.

## U4. rnnoise model `cb.rnnn`

Пользователь должен:

- Найти и скачать совместимую rnnoise model для ffmpeg `arnndn`.
- Проверить лицензию.
- Положить файл в `resources/rnnoise-models/cb.rnnn` или другой путь, который ожидает приложение.

Если модель недоступна, приложение должно уметь работать без шумоподавления, но качество может быть ниже.

## U5. Тестовые видео

Пользователь должен подготовить локальные тестовые MP4:

- короткий файл 10–30 секунд с русской речью;
- файл без аудиодорожки;
- файл с шумом;
- файл с кириллицей и пробелами в пути;
- по возможности длинный файл 30+ минут;
- для chunking — очень длинный файл или искусственно большой audio file.

Эти файлы не стоит коммитить, если они большие или содержат приватную речь.

## U6. Реальная проверка Groq

Пользователь должен вручную запустить live smoke-test:

1. Ввести Groq API key в Settings.
2. Добавить короткий MP4.
3. Проверить, что появился `.txt`.
4. Проверить качество русского текста.
5. Проверить поведение при нескольких файлах.
6. Проверить rate limit/retry, если есть возможность.

AI может подготовить manual test checklist, но не должен выполнять реальные платные вызовы без разрешения.

## U7. UX-решения владельца продукта

Пользователь должен принять решения:

- Название приложения: `VideoTranscriber`, `Slova` или другое.
- Иконка приложения.
- Дефолтная папка вывода: рядом с видео или отдельная папка.
- Поведение при конфликте файлов: suffix/overwrite/skip.
- Включать ли postprocess по умолчанию.
- Нужен ли tray mode.
- Нужно ли автообновление в первом релизе.

## U8. Code signing и публикация

Для публичного распространения пользователь должен:

- Получить/купить code signing certificate, если нужен.
- Настроить хранение signing secrets в CI.
- Настроить GitHub Releases или другой hosting для updater.
- Сгенерировать updater signing keys.
- Не коммитить private keys.

AI может подготовить конфиги с placeholders, но финальные секреты и публикация — ответственность пользователя.

## U9. Юридические и privacy-решения

Пользователь должен решить:

- Можно ли отправлять аудио в Groq с точки зрения privacy.
- Нужно ли предупреждение пользователю в UI: «Аудио отправляется в Groq API».
- Нужна ли privacy policy.
- Можно ли хранить transcript в SQLite на диске.
- Нужна ли кнопка «удалить историю и транскрипты».

## U10. Финальное ручное acceptance testing

Перед релизом пользователь должен проверить:

- Установка на чистую Windows 11 машину.
- Первый запуск без API key.
- Ввод API key.
- Drag & drop 10–50 файлов.
- Отмена задачи.
- Retry после ошибки.
- Экспорт TXT/SRT/JSON.
- Перезапуск приложения и восстановление истории.
- Удаление/отсутствие ffmpeg и понятность ошибки.
- Работа с кириллическими путями.
- Работа при нестабильном интернете.

---

# Рекомендуемый порядок выполнения

## MVP

1. Блок A — Инициализация проекта.
2. Блок B — Типы, IPC, события.
3. Блок C — SQLite, настройки, keyring.
4. Блок D — FFmpeg/ffprobe.
5. Блок E — Groq client, retry, rate limit.
6. Блок F — Core scheduler и pipeline без чанкинга.
7. Блок G — Результаты и экспорт.
8. Блок H — Основной UI очереди.
9. Блок I — Settings и transcript detail.
10. Блок J — MVP hardening.

## После MVP

11. Блок K — Кэш и дедупликация.
12. Блок L — Fallback-чанкинг и stitch.
13. Блок M — Llama postprocess.
14. Блок N — Packaging, updater, release pipeline.
15. Блок U — Пользовательские действия выполняются параллельно там, где необходимы.

---

# Матрица зависимостей

| Блок | Название | Зависит от | Можно делать без API key | Можно делать без ffmpeg binaries |
|---|---|---|---:|---:|
| A | Инициализация проекта | нет | да | да |
| B | Типы, IPC, события | A | да | да |
| C | SQLite, настройки, keyring | B | да | да |
| D | FFmpeg adapter | B, C | да | частично |
| E | Groq client | B, C, D | частично, через mocks | да |
| F | Core scheduler/pipeline | B, C, D, E | частично | частично |
| G | Results/export | B, C, F | да | да |
| H | UI queue | B, F | да | да |
| I | Settings/detail | C, G, H | да | да |
| J | Hardening | A–I | да | да |
| K | Cache/dedup | C, F, G, H | да | да |
| L | Chunking/stitch | D, E, F, G | да, через mocks | частично |
| M | Llama postprocess | E, F, G, I | частично, через mocks | да |
| N | Packaging/release | A–J | да | частично |
| U | User actions | нет | нет, для live | нет, для media |

---

# Главные риски и как их закрывать блоками

| Риск | Где закрывается | Как закрывается |
|---|---|---|
| Слабый CPU задыхается от параллельного ffmpeg | F, J | отдельный `cpu_sem`, настройки concurrency |
| Groq rate limit | E, F | retry/backoff, Retry-After, rate limiter |
| Потеря прогресса/истории после перезапуска | C, F, J | SQLite state persistence, startup recovery |
| Небезопасное хранение API key | C, J | keyring, masking, no logs |
| Ошибки при кириллических путях | D, F, U | argv process API, manual tests |
| Дубли на чанках | L | overlap + text similarity stitch tests |
| Перезапись пользовательских файлов | G, I | conflict policy, atomic write |
| UI лагает на 50 задачах | H | store by id, throttled events, virtualization |
| Реальные API вызовы в тестах | E, M | mock server, ignored live tests |
| Непонятные ошибки пользователю | B, F, J | AppError taxonomy, UI mapping |

---

# Финальный ориентир

После выполнения блоков A–J должен получиться MVP, который:

- запускается как desktop-приложение;
- принимает несколько MP4 через drag & drop;
- валидирует видео через ffprobe;
- извлекает Opus через ffmpeg;
- отправляет аудио в Groq Whisper Large v3 Turbo;
- показывает прогресс и состояния;
- сохраняет TXT рядом с видео или в выбранную папку;
- хранит историю в SQLite;
- позволяет retry/cancel;
- хранит API key в OS keychain;
- позволяет просмотреть и отредактировать transcript;
- экспортирует TXT/SRT/JSON;
- пишет логи и показывает понятные ошибки.

После выполнения блоков K–N приложение станет ближе к production-ready продукту: кэш, чанкинг больших файлов, Llama postprocess, сборка установщика и release pipeline.
