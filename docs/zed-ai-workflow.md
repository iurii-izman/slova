# VideoTranscriber — Zed AI workflow и настройка редактора

Дата подготовки: 2026-04-28

Цель документа — настроить Zed под дешёвую и эффективную разработку VideoTranscriber: максимум полезного автопилота, минимум лишних дорогих запросов, безопасная работа с файлами, понятные задачи и MCP только там, где он реально нужен.

---

## 1. Что найдено в проекте до настройки

На момент анализа в проекте не было найдено:

- `.zed/settings.json`;
- `.zed/tasks.json`;
- project-level MCP config;
- явных Zed workflow-документов;
- отдельных AI rules, кроме проектных правил, переданных в окружение.

В проект добавлены:

- `.zed/settings.json` — project-level настройки форматирования, исключения тяжёлых директорий, Rust Analyzer performance hints.
- `.zed/tasks.json` — быстрые задачи для Git, npm, UI и Rust/Tauri.
- `.rules` — project-level правила для Zed Agent.
- `.gitignore` — исключения зависимостей, build outputs, секретов и локальных файлов.
- `docs/zed-ai-workflow.md` — этот документ.

Важно: Zed может попросить доверить worktree. Для применения `.zed/settings.json` и запуска project tasks нужно нажать trust/доверить проект через индикатор в title bar или команду `workspace::ToggleWorktreeSecurity`.

---

## 2. Что можно настроить автоматически, а что нет

### Автоматически в репозитории

Можно безопасно хранить в проекте:

- project-level `.zed/settings.json`;
- project-level `.zed/tasks.json`;
- `.rules` для Agent Panel;
- документацию по workflow;
- безопасные шаблоны/примеры настроек.

### Только вручную в глобальных настройках Zed

Нельзя безопасно или технически правильно закоммитить в проект:

- реальные API keys для AI-провайдеров;
- реальные MCP tokens;
- личные model provider preferences;
- пользовательские global Zed profiles;
- tool permissions, если они должны работать во всех проектах;
- абсолютные локальные пути вне проекта;
- code signing/updater secrets.

Глобальный файл настроек Zed на Windows обычно находится здесь:

```text
%APPDATA%\Zed\settings.json
```

Открыть его проще всего через Command Palette:

- `Ctrl+Shift+P`;
- команда `zed: open settings file`.

---

## 3. Project-level настройки, которые уже добавлены

### `.zed/settings.json`

Основные решения:

- 2 пробела для TypeScript/TSX/JSON/TOML/Markdown.
- 4 пробела для Rust.
- `format_on_save` включён для кода.
- Markdown не форматируется автоматически, чтобы не ломать длинные промпты и архитектурные документы.
- Исключены тяжёлые директории: `node_modules`, `target`, `dist`, `coverage`, `.vite`, `.turbo`.
- Rust Analyzer получает отдельный target dir через `analyzerTargetDir`, чтобы не мешать обычному `target`.
- Rust Analyzer check ограничен более лёгким режимом (`allTargets=false`, `workspace=false`) для экономии CPU на слабом ноутбуке.

Почему так:

- проект будет содержать Rust + Tauri + Solid.js;
- `node_modules` и `target` резко увеличивают шум поиска и AI-контекста;
- на Ryzen 3 / 8 GB RAM нельзя позволять tooling без необходимости гонять полный workspace check на каждый save.

### `.zed/tasks.json`

Добавлены задачи:

- `Git: status`;
- `Git: diff stat`;
- `Project: npm install`;
- `Project: dev`;
- `Project: tauri dev`;
- `Project: build`;
- `Project: lint`;
- `Project: typecheck`;
- `UI:* (apps/ui)` — если будет выбран layout с `apps/ui`;
- `Rust: fmt`;
- `Rust: check`;
- `Rust: clippy`;
- `Rust: test`;
- `Rust: build release`.

Часть задач начнёт работать только после инициализации Tauri/Rust/Frontend проекта. Сейчас они нужны как готовая панель команд на будущие блоки разработки.

Запуск задач:

- `Ctrl+Shift+P`;
- `task: spawn`;
- выбрать нужную задачу.

---

## 4. Рекомендуемые global AI settings для дешёвой разработки

Ниже — шаблон для твоего глобального `%APPDATA%\Zed\settings.json`. Его нужно адаптировать под реально доступных тебе провайдеров и модели.

Главный принцип:

- default/fast профиль — дешёвая модель для большинства задач;
- architect/review профиль — сильная дорогая модель только для сложной архитектуры и финального review;
- reviewer профиль — read-only, без права редактирования и terminal, чтобы получать безопасный независимый обзор.

Пример-шаблон:

```json
{
  "agent": {
    "default_profile": "cheap-coder",
    "default_model": {
      "provider": "zed.dev",
      "model": "gpt-5-mini"
    },
    "inline_assistant_model": {
      "provider": "zed.dev",
      "model": "gpt-5-nano"
    },
    "commit_message_model": {
      "provider": "zed.dev",
      "model": "gpt-5-nano"
    },
    "thread_summary_model": {
      "provider": "zed.dev",
      "model": "gpt-5-nano"
    },
    "model_parameters": [
      {
        "temperature": 0.1
      }
    ],
    "use_modifier_to_send": true,
    "expand_edit_card": false,
    "expand_terminal_card": false,
    "enable_feedback": false,
    "profiles": {
      "cheap-coder": {
        "name": "Cheap Coder",
        "default_model": {
          "provider": "zed.dev",
          "model": "gpt-5-mini"
        },
        "tools": {
          "fetch": true,
          "thinking": true,
          "copy_path": true,
          "find_path": true,
          "delete_path": true,
          "create_directory": true,
          "list_directory": true,
          "diagnostics": true,
          "read_file": true,
          "open": false,
          "move_path": true,
          "grep": true,
          "edit_file": true,
          "terminal": true
        },
        "enable_all_context_servers": false,
        "context_servers": {
          "context7": {
            "tools": {
              "resolve-library-id": true,
              "get-library-docs": true
            }
          }
        }
      },
      "architect-review": {
        "name": "Architect / Review",
        "default_model": {
          "provider": "zed.dev",
          "model": "claude-sonnet-4-5"
        },
        "tools": {
          "fetch": true,
          "thinking": true,
          "copy_path": true,
          "find_path": true,
          "delete_path": false,
          "create_directory": false,
          "list_directory": true,
          "diagnostics": true,
          "read_file": true,
          "open": false,
          "move_path": false,
          "grep": true,
          "edit_file": false,
          "terminal": false
        },
        "enable_all_context_servers": false,
        "context_servers": {
          "context7": {
            "tools": {
              "resolve-library-id": true,
              "get-library-docs": true
            }
          }
        }
      },
      "readonly-auditor": {
        "name": "Read-only Auditor",
        "default_model": {
          "provider": "zed.dev",
          "model": "gpt-5-mini"
        },
        "tools": {
          "fetch": true,
          "thinking": true,
          "copy_path": false,
          "find_path": true,
          "delete_path": false,
          "create_directory": false,
          "list_directory": true,
          "diagnostics": true,
          "read_file": true,
          "open": false,
          "move_path": false,
          "grep": true,
          "edit_file": false,
          "terminal": false
        },
        "enable_all_context_servers": false,
        "context_servers": {}
      }
    }
  }
}
```

Если у тебя нет Zed-hosted моделей или названия моделей отличаются, выбери аналоги:

- дешёвый coding/editing: mini/nano/flash/haiku/fast-code модель;
- сложная архитектура: sonnet/opus/gpt reasoning модель;
- summaries/commit/inline: самая дешёвая nano/flash модель.

---

## 5. Tool permissions: автопилот без постоянных подтверждений, но с защитой

Оптимальная стратегия:

- чтение/поиск/редактирование файлов проекта можно разрешить;
- terminal по умолчанию подтверждать;
- безопасные проверки (`cargo check`, `npm run build`, `git status`) можно auto-allow;
- destructive/secret/git-push команды всегда подтверждать или запрещать.

Добавь или смержи этот блок в global `%APPDATA%\Zed\settings.json`:

```json
{
  "agent": {
    "tool_permissions": {
      "default": "allow",
      "tools": {
        "terminal": {
          "default": "confirm",
          "always_allow": [
            { "pattern": "^git\\s+(status|diff|log|show|branch)(\\s|$)" },
            { "pattern": "^cargo\\s+(fmt|check|test|clippy|build)(\\s|$)" },
            { "pattern": "^npm\\s+(run\\s+(build|lint|typecheck|test)|test)(\\s|$)" }
          ],
          "always_confirm": [
            { "pattern": "\\b(npm|pnpm|yarn)\\s+(install|add|remove|update)\\b" },
            { "pattern": "\\bcargo\\s+(install|update)\\b" },
            { "pattern": "\\btauri\\s+build\\b" },
            { "pattern": "\\bgit\\s+(push|reset|clean|checkout|switch|merge|rebase|commit)\\b" },
            { "pattern": "\\b(rm|del|Remove-Item)\\b" }
          ],
          "always_deny": [
            { "pattern": "rm\\s+-rf\\s+(/|~)" },
            { "pattern": "GROQ_API_KEY|OPENAI_API_KEY|ANTHROPIC_API_KEY|gsk_" }
          ]
        },
        "edit_file": {
          "default": "allow",
          "always_deny": [
            { "pattern": "(^|/|\\\\)\\.env(\\..*)?$" },
            { "pattern": "\\.(pem|key|pfx|p12)$" }
          ]
        },
        "delete_path": {
          "default": "confirm",
          "always_deny": [
            { "pattern": "transcriber-spec\\.md$" },
            { "pattern": "transcriber-architecture-analysis\\.md$" },
            { "pattern": "transcriber-autopilot-development-plan\\.md$" }
          ]
        },
        "move_path": {
          "default": "confirm"
        },
        "open": {
          "default": "confirm"
        }
      }
    }
  }
}
```

Примечание: Zed проверяет regex по строке команды terminal. Если команда составная, auto-allow сработает только если все части безопасны. Для сомнительных команд лучше оставить confirm.

---

## 6. MCP: что реально полезно для этого проекта

### Рекомендация по MCP

Для VideoTranscriber не нужно включать много MCP сразу. Чем больше MCP, тем выше шум, стоимость и вероятность, что модель выберет не тот инструмент.

Рекомендуемый минимум:

1. `context7` — актуальная документация по Tauri, Solid, Tokio, sqlx, reqwest, keyring.
2. `github` — только когда появится репозиторий и понадобятся issues/PR/release automation.
3. `puppeteer` или `playwright` — только после появления UI и E2E-сценариев.
4. `filesystem` MCP не обязателен, потому что Zed Agent уже умеет читать/редактировать файлы проекта нативными tools. Если включать — строго ограничивать `C:\Dev\slova`.

### Пример MCP config для global Zed settings

```json
{
  "context_servers": {
    "context7": {
      "command": "npx",
      "args": ["-y", "@upstash/context7-mcp"],
      "env": {}
    },
    "filesystem-slova": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:/Dev/slova"],
      "env": {}
    }
  }
}
```

`filesystem-slova` лучше не включать, если хватает встроенных инструментов Zed. Если включаешь — не давай ему доступ к `C:/Dev` целиком, только к `C:/Dev/slova`.

### MCP permissions

Для MCP tools лучше использовать confirm по умолчанию:

```json
{
  "agent": {
    "tool_permissions": {
      "tools": {
        "mcp:github:create_issue": {
          "default": "confirm"
        },
        "mcp:github:create_pull_request": {
          "default": "confirm"
        },
        "mcp:filesystem-slova:write_file": {
          "default": "confirm"
        },
        "mcp:filesystem-slova:delete_file": {
          "default": "confirm"
        }
      }
    }
  }
}
```

Точные имена MCP tools зависят от конкретного сервера. После установки проверь их в Agent Panel settings.

---

## 7. Rules и reusable prompts

### Project `.rules`

В корне проекта создан `.rules`. Zed автоматически добавляет его в Agent Panel context для этого worktree.

Задача `.rules`:

- закрепить русский язык ответов;
- напомнить стек и архитектуру;
- запретить секреты и реальные Groq-запросы в тестах;
- требовать дешёвый workflow;
- требовать запуск проверок после изменений.

### Rules Library

Для личной библиотеки правил в Zed стоит создать 3 reusable rules:

#### Rule: Cheap Implementation

Использовать для обычной разработки блока из `transcriber-autopilot-development-plan.md`.

Текст правила:

```text
Работай как дешёвый implementation-agent. Сначала прочитай релевантные документы проекта, затем реализуй только текущий блок. Не делай архитектурных развилок без необходимости. Не запускай долгие команды без пользы. После изменений запусти минимальные проверки и кратко отчитайся.
```

#### Rule: Architecture Review

Использовать перед крупными решениями.

```text
Работай как architecture reviewer. Не редактируй файлы без явного запроса. Проанализируй решение на слои, зависимости, безопасность, тестируемость, UI/UX и риски. Дай конкретные рекомендации и варианты trade-off.
```

#### Rule: Security / Privacy Review

Использовать перед Groq/keyring/ffmpeg/release блоками.

```text
Проведи security/privacy review. Проверь секреты, логи, внешние API, работу с файлами, shell injection, Windows paths, хранение транскриптов, тесты без реальных API calls. Не предлагай хранить секреты в репозитории.
```

Открыть Rules Library:

- Agent Panel;
- меню `...`;
- `Rules...`;
- либо command palette `agent: open rules library`.

---

## 8. Дешёвый workflow разработки по блокам

### Шаг 1. Выбрать блок

Открыть `transcriber-autopilot-development-plan.md` и выбрать следующий блок:

- A — каркас;
- B — типы/IPC;
- C — SQLite/keyring;
- D — ffmpeg;
- E — Groq;
- F — pipeline;
- и так далее.

### Шаг 2. Запустить cheap profile

В Agent Panel выбрать профиль `Cheap Coder` или аналогичный дешёвый профиль.

Дать промпт из нужного блока почти без изменений. Добавить только текущий статус проекта, если он важен.

### Шаг 3. Ограничить область

В начале запроса явно указать:

- какие файлы можно менять;
- что нельзя трогать;
- какие проверки обязательны;
- не делать реальных Groq-запросов.

### Шаг 4. После реализации

Запустить Zed tasks:

- Rust changes: `Rust: fmt`, `Rust: check`, позже `Rust: clippy`, `Rust: test`.
- Frontend changes: `Project: typecheck`, `Project: lint`, `Project: build`.
- Документы: достаточно проверки глазами + Git diff.

### Шаг 5. Review

Для крупных блоков после реализации переключиться на `Read-only Auditor` или `Architect / Review` и попросить:

- найти архитектурные ошибки;
- проверить безопасность;
- проверить соответствие `transcriber-architecture-analysis.md`;
- не редактировать файлы, только отчёт.

### Шаг 6. Дорогие модели использовать только здесь

Дорогая reasoning-модель нужна только для:

- выбора архитектурной развилки;
- сложной отладки async pipeline;
- security review перед release;
- финального review после блока F/J/L/N.

Обычные задачи — дешёвая модель.

---

## 9. Рекомендуемый Zed layout во время разработки

Удобная раскладка:

- Слева: Project Panel.
- Центр: код.
- Справа: Agent Panel.
- Снизу: Terminal/Tasks.

Постоянно открытые файлы:

- `transcriber-autopilot-development-plan.md`;
- `transcriber-architecture-analysis.md`;
- текущий файл реализации;
- diagnostics panel.

Для каждого блока держать отдельный Agent thread. Не мешать обсуждение архитектуры, реализацию и review в одном длинном треде — это увеличивает стоимость и ухудшает контекст.

---

## 10. Практические команды Zed

Полезные команды:

- `Ctrl+Shift+P` → Command Palette.
- `Ctrl+Shift+A` → Agent Panel.
- `Ctrl+P` → Go to file.
- `Ctrl+Shift+F` → Project search.
- `task: spawn` → запуск project tasks.
- `zed: open project settings` → открыть `.zed/settings.json`.
- `zed: open project tasks` → открыть `.zed/tasks.json`.
- `zed: open settings file` → открыть global settings.
- `agent: open settings` → настройки Agent Panel.
- `agent: open rules library` → Rules Library.

---

## 11. Риски и как их снижать

| Риск | Как снижать |
|---|---|
| Дорогие модели используются для мелких правок | Default profile = cheap, дорогой профиль только вручную |
| AI запускает опасные команды | Terminal default confirm + deny для secret/destructive patterns |
| MCP создаёт шум и удорожает контекст | Включать только Context7 на старте |
| Zed сканирует `target`/`node_modules` | `file_scan_exclusions` в `.zed/settings.json` |
| Rust Analyzer грузит CPU | `allTargets=false`, `workspace=false`, manual tasks для full check |
| Секреты попадают в репозиторий | `.gitignore`, `.rules`, keyring-only policy |
| Тесты делают реальные Groq requests | `.rules`, mock server requirement, ignored/manual live tests |
| AI ломает архитектуру ради lint | `.rules` запрещает упрощать архитектуру ради прохождения checks |

---

## 12. Когда менять настройки

После блока A можно уточнить `.zed/tasks.json` под фактическую структуру:

- если Tauri создал root `package.json`, оставить `Project:*` tasks;
- если UI живёт в `apps/ui`, оставить `UI:*` tasks;
- если Cargo workspace будет в root, заменить `--manifest-path src-tauri/Cargo.toml` на workspace-команды.

После появления frontend formatter можно добавить Prettier:

```json
{
  "languages": {
    "TypeScript": {
      "formatter": {
        "external": {
          "command": "npx",
          "arguments": ["prettier", "--stdin-filepath", "{buffer_path}"]
        }
      }
    },
    "TSX": {
      "formatter": {
        "external": {
          "command": "npx",
          "arguments": ["prettier", "--stdin-filepath", "{buffer_path}"]
        }
      }
    }
  }
}
```

До появления `prettier` в `devDependencies` это лучше не включать, иначе formatting может стать медленным или нестабильным.

---

## 13. Итоговый рекомендуемый workflow

1. Открыть проект `slova` в Zed и доверить worktree.
2. Проверить, что `.rules` подхватился Agent Panel.
3. В global settings настроить cheap/default модель и tool permissions.
4. Подключить только `context7` MCP, если нужна актуальная документация.
5. Работать по блокам из `transcriber-autopilot-development-plan.md`.
6. Большинство реализации делать на cheap profile.
7. Дорогую модель использовать только для архитектуры, сложной отладки и финального review.
8. После каждого блока запускать соответствующие Zed tasks.
9. Не отправлять реальные Groq-запросы в тестах без ручного разрешения.
10. Не хранить API keys в проекте.
