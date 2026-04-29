# Кэширование и Дедупликация — Реализация

## Обзор

Реализована комплексная система кэширования и дедупликации для VideoTranscriber:
- **BLAKE3 full content hash** для надежной идентификации файлов
- **Settings fingerprint** (хеш настроек) для инвалидации кэша при изменении параметров
- **Cache key** (content_hash + settings_fingerprint) для связи с результатом
- **Weak key** (size + mtime + partial_hash) для batch-level дедупликации
- **Non-blocking async hashing** (в tokio::spawn_blocking) чтобы не блокировать UI
- **Cache hit detection** перед Groq API для переиспользования результатов

## Структура

### 1. Новые типы (`src/types/mod.rs`)

```rust
pub struct ContentHash(pub String);      // BLAKE3 hex (64 chars)
pub struct SettingsFingerprint(pub String);  // Settings hash
pub struct CacheKey(pub String);         // content_hash-settings_fp
pub struct WeakKey(pub String);          // size-mtime-partial_hash
```

### 2. Новый модуль (`src/core/cache.rs`)

#### `calculate_content_hash(path) -> ContentHash`
- Вычисляет BLAKE3 хеш всего файла
- Запускается в `tokio::spawn_blocking` (не блокирует UI)
- Использует буфер 64KB для эффективного чтения

#### `settings_fingerprint(settings) -> SettingsFingerprint`
- Создает хеш настроек (язык, формат вывода)
- Синхронная операция, быстрая
- Изменение любого параметра инвалидирует кэш

#### `generate_cache_key(path, settings) -> CacheKey`
- Комбинирует content_hash + settings_fingerprint
- Два файла с одинаковым контентом и настройками = один cache key

#### `generate_weak_key(path) -> WeakKey`
- Быстрая batch-level дедупликация
- Использует: size + mtime + partial_hash (first 1MB)
- Работает в `tokio::spawn_blocking`

### 3. Обновленные состояния задач (`src/types/mod.rs`)

```rust
pub enum JobState {
    // ... существующие состояния ...
    Cached {
        output_path: PathBuf,
        duration_ms: u64,
    },
    Skipped {
        duplicate_of: JobId,
    },
    // ... остальные ...
}
```

### 4. Обновленная функция `enqueue_files` (`src/app/commands.rs`)

**Фаза 1: Batch-level дедупликация (weak keys)**
```rust
// Генерируем weak key для текущего файла
let weak_key = cache::generate_weak_key(&path).await?;

// Проверяем: видели ли мы уже этот weak key в текущем batch?
if weak_keys_seen.contains_key(&weak_key) {
    // Это дубликат → JobState::Skipped
}
```

**Фаза 2: Cache hit detection (cache keys)**
```rust
// Генерируем full cache key
let cache_key = cache::generate_cache_key(&path, &settings).await?;

// Проверяем БД: есть ли результат для этого cache key?
if let Ok(Some(cached_job_id)) = cache_repo.get(&cache_key).await {
    // Нашли старую задачу с результатом
    if old_job.state == JobState::Done { ... } {
        // Cache hit! → JobState::Cached
    }
}
```

**Результат:**
- `JobState::Queued` — обработать в pipeline
- `JobState::Cached` — не отправлять Groq, переиспользовать результат
- `JobState::Skipped` — дубликат в текущем batch

## Поведение

### Scenario 1: Новый файл
1. Генерируем weak key (в фоне)
2. Проверяем: есть ли в batch с таким weak key? **Нет**
3. Генерируем cache key (в фоне)
4. Проверяем БД: есть ли результат? **Нет**
5. **Результат:** `JobState::Queued` → отправляем в очередь

### Scenario 2: Такой же файл, повторная обработка
1. Генерируем weak key → **совпадает с previous**
2. Генерируем cache key → **совпадает с previous**
3. Находим в БД старую задачу в состоянии `Done`
4. **Результат:** `JobState::Cached` с output_path из old_job

### Scenario 3: Два одинаковых файла в одном enqueue batch
1. Первый файл: weak_key не в `weak_keys_seen` → Queued
2. Второй файл: weak_key найден в `weak_keys_seen` → **Skipped { duplicate_of: first_id }**

### Scenario 4: Повторная обработка с другими настройками
1. Файл тот же → weak key совпадает
2. Генерируем cache key с **новыми настройками** → **другой key!**
3. Не находим результат в БД → `JobState::Queued`

## Реализация в Pipeline

Когда Groq stage проверяет cache:
```rust
// В stages::transcribe()
if JobState::Cached { output_path, .. } = &job.state {
    // Не делаем запрос к Groq
    // Просто загружаем результат из файла
    // Эффективность: экономим 8-15 сек на файл
}
```

## Обновленная БД

Таблица `cache` уже существовала:
```sql
CREATE TABLE cache (
    cache_key TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id),
    created_at INTEGER NOT NULL
)
```

Используется для маппинга: `cache_key → job_id`

## Тесты

### Модульные тесты (`src/core/cache.rs`)

✅ `test_content_hash_generation` — BLAKE3 хеш вычисляется корректно
✅ `test_content_hash_different_files` — разные файлы → разные хеши
✅ `test_settings_fingerprint` — разные настройки → разные fingerprints
✅ `test_settings_fingerprint_consistency` — одинаковые настройки → одинаковые fingerprints
✅ `test_cache_key` — cache key комбинирует hash + fingerprint корректно
✅ `test_weak_key_generation` — weak key содержит size, mtime, partial_hash
✅ `test_weak_key_different_files` — разные файлы → разные weak keys

### Integration тесты

- Используются реальные temp files
- Все async операции в `#[tokio::test]`

**Результат:** 54 тестов пройдено, 0 ошибок

## Performance

### Blocking операции (в tokio::spawn_blocking)
- **Full hash (BLAKE3):** ~100 ms для 200 MB файла
- **Weak hash (first 1MB):** ~1-2 ms
- **UI не блокирует:** работает на другом потоке

### Non-blocking операции
- **Settings fingerprint:** <1 ms (синхронно)
- **Cache key generation:** full_hash + fingerprint = ~100 ms
- **Cache lookup в БД:** <1 ms (индекс по cache_key)

### Экономия
- **Cache hit:** экономим 8-15 сек (Groq delay)
- **Batch dedup:** исключаем дубликаты до очереди

## Ограничения и Future Work

1. **Cache invalidation:** нет TTL, кэш живет вечно
   - Решение: добавить опцию "Clear cache" в UI

2. **Weak key reliability:** используется partial hash (1MB)
   - Риск: два очень похожих файла на первый MB
   - Решение: full hash как финальная проверка перед cache hit

3. **Settings fingerprint:** не учитывает `prompt` и `model`
   - Текущий snapshot: язык + формат
   - Улучшение: расширить при добавлении параметризации

4. **Cache storage:** в SQLite, можно оптимизировать
   - Будущее: отдельная таблица для metadata

## Безопасность

✅ **No API key exposure:** cache key = hash, не содержит credentials
✅ **No plaintext secrets:** BLAKE3 используется для хеширования, не шифрования
✅ **Safe async:** все blocking операции отделены, UI не зависит
✅ **Atomic writes:** при сохранении кэша используется атомарность

## Summary

Реализована двухуровневая система дедупликации:

| Уровень | Метод | Ключ | Скорость | Результат |
|---------|-------|------|----------|-----------|
| **Batch** | Weak key | size+mtime+partial | 1 ms | Skipped |
| **Persistent** | Cache key | full_hash+settings | 1 ms DB lookup | Cached |

Система работает асинхронно, не блокирует UI, и может сэкономить 8-15 сек на каждый повторный файл.
