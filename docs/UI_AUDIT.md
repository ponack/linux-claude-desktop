# UI Audit — v0.9.8 (Phase 17)

**Date:** 2026-05-16 · **Audit type:** Read-only inventory, no code changes
**Purpose:** Catalog accumulated UI surface drift after 17 feature phases, to inform a phased redesign toward v1.0.0.

---

## 1. Component inventory

23 Svelte files under `src/lib/` (artifacts subfolder included). The five heaviest carry most of the surface area:

| Component | Lines | Role |
|---|---:|---|
| `Settings.svelte` | ~2,860 | 17 settings sections in one file |
| `Chat.svelte` | ~1,780 | Main chat: streaming, artifacts, agent mode, voice, offline queue, toolbar |
| `GitView.svelte` | ~970 | Git diff/log/file browser |
| `Sidebar.svelte` | ~900 | Conv list, search, sync status, menu |
| `ExtensionsView.svelte` | ~850 | MCP install/remove flows |
| `MessageBubble.svelte` | ~785 | Markdown render, code actions, edit/fork/regenerate, annotations |
| `ArtifactPanel.svelte` | ~670 | Side panel: preview/edit/history |
| `ComparisonView.svelte` | ~620 | Multi-model comparison |
| `ComputerUseView.svelte` | ~530 | CU event timeline |
| `CommandPalette.svelte` | ~370 | Global cmd/search overlay |
| `TerminalView.svelte` | ~270 | PTY shell |
| `VersionHistory.svelte` | ~200 | Artifact version timeline |
| `QuickAsk.svelte` | ~200 | Quick-ask floating widget |
| Renderers (Markdown/Mermaid/Code/Artifact*) | <150 each | Artifact renderers |
| `Toast.svelte`, `ChatWindow.svelte`, etc. | <100 | Small utilities |

**Drift signals:**
- `Settings.svelte` is a 2.8k-line monolith. Section markup is heavily repeated (each section is hand-rolled HTML).
- Floating UI is implemented at least 4 different ways (see §8).
- Message-action affordances overlap between `MessageBubble` and `Chat` (edit/fork/regenerate appear in both, styled differently).

---

## 2. CSS custom property usage

`src/styles/global.css` defines the token system. Counted ~18 named tokens covering bg/surface/text/accent/danger/success/border/code/scrollbar. Three themes are wired (dark default, `prefers-color-scheme: light`, `prefers-contrast: more` for high contrast).

**Synonym / overlap issues:**
- `--text-primary` / `--text-secondary` / `--text-muted` are semantic but underused — many components reach for `var(--text-muted)` for body copy instead of `--text-primary`.
- `--accent` and `--danger` resolve to nearly identical reds in dark mode (`#e94560` family). Semantic intent gets confused.
- `--code-bg` and `--code-inline-bg` use the same `rgba(0,0,0,0.3)` in dark mode.

**Missing tokens:**
- No `--overlay-bg` (modal backdrops). Every modal hardcodes `rgba(0,0,0,0.5)`.
- No `--live-color` (LIVE-session red is `#e94560` typed inline 8+ times).
- No `--white` / `--shadow-*` / `--radius-*` scale.
- No spacing scale (4/8/12/16/24) — padding/margin values are ad-hoc.

---

## 3. Magic values

Spot-counted ~110 hardcoded color/spacing values outside tokens. High-frequency offenders:

- `rgba(233, 69, 96, *)` — 8+ instances across `Chat.svelte` live panel, `ComputerUseView`. Should be `--live-color` + opacity utility.
- `rgba(0, 0, 0, 0.5)` — every modal backdrop. Single `--overlay-bg` would consolidate.
- `#ffffff` / `#fff` — sprinkled across `ArtifactPreview`, `ComparisonView`, `ComputerUseView` (5+ instances).
- `#ef4444`, `#f59e0b` in `ComparisonView` — Tailwind-style reds/yellows that don't match the theme's palette.

**Spacing magic:** button padding ranges across `6px`, `8px`, `10px 14px`, `4px 10px`, `3px 10px`. No discernible scale.

**Estimated leverage:** introducing ~8 new tokens + a 4-step spacing scale would eliminate ~40-50 of these.

---

## 4. Settings sections (17)

Order matches the `sections` array in `Settings.svelte:11–29`:

1. `general` — system prompt, theme, custom CSS, update interval
2. `appearance` — theme & display
3. `prompts` — prompt library CRUD
4. `projects` — project context
5. `integrations` — MCP servers
6. `schedules` — scheduled prompts
7. `endpoints` — custom LLM endpoints
8. `routing` — model routing rules
9. `knowledge` — knowledge base + file watching
10. `data` — usage/cost analytics + DB management
11. `accessibility` — font size, motion, contrast
12. `computeruse` — computer-use model + availability
13. `git` — default repo, availability
14. `sync` — git/WebDAV/S3 sync
15. `apiserver` — local REST API, QR pairing, LAN access
16. `plugins` — plugin discovery / install
17. `about` — version, arch, OS

**Organization debt:**
- `general` and `appearance` overlap (theme lives in general, but display knobs are in appearance).
- `accessibility` overlaps with `appearance` (font size lives in both contextually).
- `sync` / `apiserver` / `plugins` are late-phase additions; they belong under an "Advanced" or "Integrations" umbrella, not as siblings of "General".
- No search-in-settings — 17 flat sections is past the point where users can scan.

**Proposed grouping (for later PR, not this audit):**
`Account` (general, appearance, accessibility, about) · `Workflow` (prompts, schedules, routing, projects, knowledge) · `Models` (endpoints, computeruse) · `Integrations` (git, sync, apiserver, plugins, integrations) · `Data` (data).

---

## 5. Chat toolbar density

`Chat.svelte:960–990` — current toolbar actions:

1. Popout (open in new window)
2. Export `.md`
3. Export `.json`
4. Share (copy link)
5. Live session toggle
6. Project selector (conditional dropdown)

Plus, in the same toolbar row: model indicator (label), token usage bar (~60px wide w/ in/out/cost labels), conversation cost.

**Issue:** 6 icon buttons + a label + a usage bar in one row. On narrower windows it wraps or truncates. Export `.md` and `.json` are textually labeled while others are icon-only — inconsistent affordance.

**Likely fix:** Group `.md`/`.json`/share/live under a single overflow menu (kebab); leave only popout + project selector + usage at top level.

---

## 6. Hover-only interaction patterns (touch-broken)

Three places use `:hover` to reveal otherwise-invisible controls. None of them have a focus-visible fallback, so they're unreachable by keyboard AND broken on touch:

1. `MessageBubble.svelte` — `.message:hover .message-actions { opacity: 1 }`. Hides edit/regenerate/fork/read-aloud/annotate.
2. `MessageBubble.svelte` — `.annotation:hover .annotation-delete { opacity: 1 }`. Hides the per-note ✕.
3. `GitView.svelte` — `.file-row:hover .file-action-btn { opacity: 1 }`. Hides stage/discard buttons.

**Severity:** high. Touch users (the companion PWA viewer is a phone) cannot use these affordances at all.

---

## 7. Focus / ARIA gaps

Spot-checked 15+ buttons across Chat, MessageBubble, Sidebar, CommandPalette.

**Present (good):**
- Chat textarea has `aria-label`
- All Chat toolbar buttons have `aria-label`
- MessageBubble actions have `aria-label` per action
- CommandPalette has `role="dialog" aria-label="..."`
- Messages have `role="article" aria-label="[role] message"`
- Skip link present in Settings

**Gaps:**
- Stop button in Chat composer has no `aria-label`
- Some dialogs use `role="dialog"` but miss `aria-modal="true"`
- Live-session pulsing dot has no `aria-hidden="true"`; screen readers will announce it
- Code-block action buttons (Copy/Run/Artifact) are imperatively `appendChild`'d in `MessageBubble.attachCopyButtons()` — fragile, not re-rendered by Svelte, may lose focus state on content updates
- Global `:focus-visible { outline: 2px solid var(--accent) }` is set, but `outline-offset` clips on tightly-packed icon buttons

---

## 8. Modal / panel / popover patterns

Four implementations of "floating UI" exist. None share components:

1. **Centered modal** (`Chat.svelte` variable-overlay, URL import): `rgba(0,0,0,0.5)` backdrop, fixed position, flex-centered, dialog with `border-radius: 12px` + custom shadow.
2. **Right-side slide panel** (`ArtifactPanel.svelte`): absolute right, 40% width, full-height border-left.
3. **Top-centered palette** (`CommandPalette.svelte`): same backdrop, fixed top area, max-height 60vh.
4. **Confirmation dialog** (`ExtensionsView.svelte`): own `.dialog-backdrop` + `.dialog` styles, 500px width.

**Inconsistencies:**
- Backdrop color: hardcoded everywhere
- Border radius: 12px in three places, 8px in Toast
- Z-index spread: 1800–2000 with no registry
- Close affordances: some have an X, some only ESC, some click-outside; behavior varies
- Shadows: applied in one, missing in three

---

## 9. Icon usage

Every icon is inline SVG. Spot-checked sizes & stroke widths:

| Surface | width × height | stroke-width |
|---|---:|---:|
| Chat toolbar | 14 × 14 | 2 |
| Chat composer attachment buttons | 18 × 18 | 2 |
| MessageBubble actions | 13 × 13 | 2 |
| Settings sidebar nav | rendered at parent size | varies |
| Annotation delete | 10 × 10 | 2.5 |
| Modal close buttons | varies | varies |

Same icon (e.g. "X close") is reimplemented at least twice with different sizes. No icon component exists.

---

## 10. Empty / loading / error states

Sample audit of state coverage:

| View | Empty | Loading | Error |
|---|---|---|---|
| Chat (no messages) | ✓ "Start a conversation…" panel | ✓ load-more sentinel | ✓ error bubble role |
| Sidebar conv list | ✗ blank if empty | ✗ silent | ✗ console-only |
| GitView panels | ✗ blank | ✓ "Loading…" text | ✗ silent |
| Settings → knowledge / projects / MCP | ✗ blank list | ✗ no spinner | ✗ silent |
| ComparisonView | ✓ "No previous comparisons" | partial | ✓ "No API keys configured" |

~50% coverage. Most async loads have no skeleton or spinner; users see flashes of empty UI. No standardized `<Spinner />` / `<Skeleton />` / `<EmptyState />` component.

---

## Priority fixes (recommended order)

These are the highest-leverage changes the audit surfaced, ordered by ratio of impact to effort. Each becomes its own PR in Act 2/3 of the redesign plan.

1. **Design tokens pass** — add the ~8 missing tokens (`--overlay-bg`, `--live-color`, `--white`, `--shadow-1/2/3`, `--radius-1/2/3`, spacing scale). Sweep components to replace hardcoded values. High impact, mechanical work.
2. **Shared modal primitive** — one `<Modal>` component with backdrop, z-index from a registry, consistent close affordances. Migrate the 4 existing implementations.
3. **Icon component** — `<Icon name="..." size={14} />` reading from an SVG registry. Replaces ~30 inline reimplementations.
4. **Hover-action fix** — add `:focus-within` and `:focus-visible` selectors to the three hover-reveal patterns; touch tap should also reveal.
5. **Settings reorganization** — group 17 sections into ~5 buckets with collapsible nav; add search.
6. **Toolbar overflow menu** — collapse `.md`/`.json`/share/live into a kebab.
7. **Standardized loading/empty states** — `<Spinner />`, `<EmptyState>` components; wire to async Settings sections + Sidebar.
8. **Code-block action buttons** — port the imperative `appendChild` pattern in `MessageBubble.attachCopyButtons()` to a Svelte-managed component.

UI consistency feels like roughly 60–65%. Two design sprints of mechanical work (tokens + components) plus one of surface refinement should land v1.0.0 cleanly.
