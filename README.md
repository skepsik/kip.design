# wiki-kit

Шаблон design-wiki на VitePress: контент в `content/`, деплой на VPS (WireGuard-only nginx), bootstrap новых вики.

**Не npm-пакет** — git template + скрипты. Каждый проект получает свой репозиторий `{slug}.design`.

## Что внутри

| Путь | Назначение |
|------|------------|
| `.wiki.json` | `slug`, `title`, `description`, `github` → `base` и метаданные VitePress |
| `.vitepress/config.ts` | читает `.wiki.json`, noindex, local search |
| `content/` | markdown-канон |
| `templates/` | каркас новой страницы (generic) |
| `scripts/new-wiki.sh` | создать `{slug}.design` из kit |
| `scripts/setup-vps-wiki-nginx.sh` | разовый nginx hub на WG IP |
| `scripts/wiki-nginx-add.sh` | добавить `/{slug}/wiki/` на VPS |
| `.github/workflows/deploy-wiki.yml` | build → SCP → VPS |

## Новая wiki

```bash
cd wiki-kit
./scripts/new-wiki.sh secrets-vault "Secrets Vault design" \
  --github https://github.com/skepsik/secrets-vault.design

cd ../secrets-vault.design
npm install
npm run docs:dev
# http://localhost:5173/secrets-vault/wiki/
```

Дальше:

1. Правь `content/`, sidebar в `.vitepress/config.ts`.
2. При необходимости — свои `templates/` (стиль §, lifecycle с issues).
3. `git remote add origin …`, push в `master`.
4. Secrets в GitHub: `VPS_HOST`, `VPS_USER`, `VPS_SSH_KEY`, `VPS_SSH_PORT`, опционально `VPS_WIKI_PATH`.

## VPS (multi-wiki)

Разово на сервере:

```bash
bash scripts/setup-vps-wiki-nginx.sh
```

На каждую wiki:

```bash
bash scripts/wiki-nginx-add.sh utlas
bash scripts/wiki-nginx-add.sh secrets-vault
```

По умолчанию root: `$HOME/{slug}-wiki/www`. Override: второй аргумент или secret `VPS_WIKI_PATH`.

URL: `http://10.243.63.1/{slug}/wiki/` (только WireGuard).

### Миграция с utlas.design

Старый `sites-enabled/utlas-wiki` отключается при `setup-vps-wiki-nginx.sh`. Переподключи:

```bash
bash scripts/wiki-nginx-add.sh utlas "$HOME/utlas-wiki/www"
```

## Локальная разработка (сам kit)

```bash
npm install
npm run docs:dev
```

Kit ships с `.wiki.json` slug=`example` — только для проверки сборки.

## `.wiki.json`

```json
{
  "slug": "my-project",
  "title": "My Project design",
  "description": "Канонический design для My Project",
  "lang": "ru-RU",
  "github": "https://github.com/OWNER/my-project.design"
}
```

`slug` задаёт URL: `/{slug}/wiki/`.

## Что остаётся per-project

- весь `content/`
- sidebar (ручной, под домен)
- опционально свои content-conventions в `templates/README.md`
- связь с issues родительского репо (ops rules, не в kit)

## Связь с кодом

Как `utlas.design` ↔ `utlas-ts`:

```bash
cd utlas-ts
git clone git@github.com:skepsik/utlas.design.git design
# или symlink: design → ../utlas.design
```

Родительский репо держит `design/` в `.gitignore`.
