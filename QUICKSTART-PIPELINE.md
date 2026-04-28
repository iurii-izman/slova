# VideoTranscriber Pipeline — Quick Start

## 🚀 Базовое использование

### 1. Инициализация (в main.rs)

```rust
// State инициализируется в setup() асинхронно
let app_state = AppState::new(db_path, groq_api_key).await?;
app.manage(Arc::new(RwLock::new(Some(app_state))));
```

### 2. Enqueue файлов (через Tauri)

```typescript
// Frontend
const jobIds = await invoke('enqueue_files', { 
  paths: ['/path/to/video.mp4'] 
});
```

**Result**: Job'ы созданы в БД, добавлены в очередь scheduler'а

### 3. Запуск scheduler (в background)

```rust
// После инициализации state в main.rs
tauri::async_runtime::spawn(async move {
    if let Some(state) = state.read().await.as_ref() {
        let _ = state.scheduler.run().await;
    }
});
```

### 4. Слушать events (Frontend)

```typescript
import { listen } from '@tauri-apps/api/event';

listen('queue:tick', (event) => {
  const tick = event.payload as QueueTick;
  tick.updates.forEach(update => {
    console.log(`Job ${update.id}: ${update.state.kind}`);
  });
});
```

---

## 📊 State Transitions

```
Queued
   ↓ (scheduler picks up)
Probing (ffprobe validate)
   ↓ (valid audio found)
Extracting { progress: 0.5 } (ffmpeg opus)
   ↓ (upload ready)
Uploading { progress: 0.0, chunk_idx: 1, chunk_total: 1 }
   ↓ (upload complete)
Transcribing { chunk_idx: 1, chunk_total: 1 }
   ↓ (API response)
Done { output_path: "/path/to/video.txt", duration_ms: 1800000 }

OR on error:
Failed { error: AppErrorView { code, message }, attempts: 1 }
   ↓ (if retryable)
[back to Queued for retry]
```

---

## 🔧 Управление очередью

### Отмена одного job'а
```typescript
await invoke('cancel_job', { id: jobId });
```
→ `CancellationToken` для job'а активируется → pipeline выходит из stage

### Повтор после ошибки
```typescript
await invoke('retry_job', { id: jobId });
```
→ State сбросится на Queued → передобавится в очередь

### Пауза всей очереди
```typescript
await invoke('pause_queue');
```
→ Текущие job'ы завершат текущий stage, новые ждут

### Возобновление
```typescript
await invoke('resume_queue');
```

---

## 🎯 Доступ к результатам

### Получить текст транскрипции
```typescript
const transcript = await invoke('get_transcript', { id: jobId });
console.log(transcript.text);
```

### Отредактировать текст
```typescript
await invoke('save_transcript_edit', { 
  id: jobId, 
  text: "Corrected text..." 
});
```

### Экспортировать в формат
```typescript
const path = await invoke('export', { 
  id: jobId, 
  format: 'txt' // | 'srt' | 'json'
});
```

---

## 📈 Мониторинг

### Получить список всех job'ов
```typescript
const jobs = await invoke('list_jobs', { 
  filter: { state: 'Done', limit: 10 }
});
```

### Health check
```typescript
const status = await invoke('health_check');
// { ok: true, version: "0.1.0" }
```

---

## ⚙️ Параметры и конфиг

### API Key (один раз)
```typescript
await invoke('save_api_key', { key: 'gsk_...' });
```
→ Сохраняется в OS keychain, загружается при запуске

### Settings
```typescript
const settings = await invoke('get_settings');
// { language: 'ru', output_format: 'Txt', parallelism: 3, ... }

await invoke('set_settings', { 
  ...settings, 
  parallelism: 5 
});
```

---

## 🐛 Debugging

### Логи из Rust
```bash
# Windows PowerShell
$env:RUST_LOG="slova_tauri=debug"; cargo run --features with_tauri

# Linux/macOS
RUST_LOG=slova_tauri=debug cargo run --features with_tauri
```

### Проверка состояния job'а в БД
```bash
sqlite3 ~/.app-data-dir/transcriber.db \
  "SELECT id, display_name, state FROM jobs LIMIT 5;"
```

---

## 🚨 Обработка ошибок

### Типичные ошибки

| Ошибка | Причина | Решение |
|--------|---------|---------|
| `AUTH_FAILED` | Нет API key | Вызвать `save_api_key()` |
| `RATE_LIMIT` | Много запросов | Автоматический retry (100ms-30s) |
| `NETWORK_ERROR` | Нет интернета | Retry через exponential backoff |
| `INVALID_FILE` | Некорректный MP4 | Проверить файл через ffprobe |
| `FS_ERROR` | Ошибка записи | Проверить права и место на диске |

### Все ошибки приходят в state
```rust
JobState::Failed {
  error: AppErrorView {
    code: "RATE_LIMIT",
    message: "Rate limited. Retry after 60s",
    details: None
  },
  attempts: 1
}
```

---

## 📝 Пример workflow'а

```typescript
// 1. User выбирает файлы
const files = ['/path/to/video1.mp4', '/path/to/video2.mp4'];

// 2. Enqueue
const jobIds = await invoke('enqueue_files', { paths: files });
console.log(`Enqueued ${jobIds.length} jobs`);

// 3. Слушать обновления
listen('queue:tick', (event) => {
  const { updates, ts } = event.payload;
  updates.forEach(upd => {
    console.log(`[${ts}] Job ${upd.id.substring(0,8)}: 
      ${upd.state.kind} 
      ${upd.bytes_uploaded || 'N/A'} bytes`);
  });
});

// 4. Ждать завершения
async function waitForDone(jobId, timeout = 300000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const jobs = await invoke('list_jobs', {});
    const job = jobs.find(j => j.id === jobId);
    if (job?.state.kind === 'Done') {
      return job.state.output_path;
    }
    await new Promise(r => setTimeout(r, 1000));
  }
  throw new Error('Timeout');
}

// 5. Получить результат
const outputPath = await waitForDone(jobIds[0]);
const transcript = await invoke('get_transcript', { id: jobIds[0] });
console.log(transcript.text);

// 6. (Optional) Отредактировать и экспортировать
const edited = transcript.text.replace('bad', 'good');
await invoke('save_transcript_edit', { 
  id: jobIds[0], 
  text: edited 
});
const finalPath = await invoke('export', { 
  id: jobIds[0], 
  format: 'txt' 
});
```

---

## 🎓 Архитектурные решения

### Почему именно так?

**Семафоры (2 CPU, 3 Network)**
- Ryzen 3 хорошо работает с 2-3 конкурентными потоками
- Groq free tier: 30 RPM ≈ 2 req/sec безопасно

**Exponential backoff с jitter**
- Избегает thundering herd при rate limit
- Jitter ±10% разбивает retry пики

**Atomic write через temp file**
- Гарантирует не-poврежденный результат
- Атомичная операция на FS

**No fallback-chunking в MVP**
- Большинство видео < 100MB после opus кодирования
- Сложность кодирования > выигрыш

**SQLite вместо Redis**
- No external dependency, все в коробке
- История сохраняется между перезапусками

---

## 📚 Related Docs

- [`transcriber-spec.md`](./transcriber-spec.md) — техническая спецификация
- [`transcriber-architecture-analysis.md`](./transcriber-architecture-analysis.md) — детальная архитектура
- [`CYCLE-2-COMPLETION.md`](./CYCLE-2-COMPLETION.md) — полный отчет о реализации

---

**Last Updated**: 2024-04-29  
**Version**: 0.1.0  
**Status**: ✅ Ready to deploy
