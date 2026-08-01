# UI design principles

## Icons

- Interactive icons must be SVGs from the shared UI icon renderer or the existing SVG mask set.
- Do not use Unicode characters, emoji, or text glyphs as icons. Their shape, alignment, and availability vary across Windows, macOS, browsers, and fallback fonts.
- Text remains appropriate for labels, status values, scientific notation, and keyboard hints such as `↑↓` or `⌘K`.
- Icon-only controls must retain an accessible `title` or `aria-label`.

## Buttons

- Standalone CTAs use `.btn-primary` / `.btn-ghost` from `ui/src/styles/base.css`.
- Toolbar rows that already own chrome (modal/settings `.row`, plugin toolbar, plan/approval actions, file retry) use `button.primary` for the filled clay look; do not redefine clay fills per surface.
- Do not use bare `button.primary` for sidebar nav — `.side-btn.primary` is a soft affordance, not a filled CTA.

## Spacing, type, and radius

- Prefer `--space-1`…`--space-7`, `--text-xs`…`--text-display`, and the three radius tiers (`--radius-xs` / `--radius-sm` / `--radius`) from `base.css`.
- Map near-miss radii onto those tiers (6–9→xs, 10–14→sm, 16–22→lg). Keep `999px` pills, `50%` circles, and asymmetric chat bubbles as literals.
- Adopt the scale first on brand surfaces (projects landing, chat empty, research graph); avoid one-off px when extending those surfaces.

## Brand surfaces

- Projects landing keeps a serif hero title with the logo mark and a soft clay wash — not a dashboard of promo cards.
- Chat empty and research-graph empty reuse the logo treatment (`.empty-logo` / `.rp-empty-icon.brand`) instead of dashed placeholders.
- Research graph headings use Source Serif at `--text-lg`; list/canvas stay utilitarian.

## Composer attachments and references

- Files, images, skills, artifacts, conversations, execution environments, and runtime references must remain visually distinguishable before and after send.
- Image attachments use a real thumbnail when the project file is readable. Other files use a document card with a filename and type label.
- Persisted transcript markers such as `Uploaded files:` and `Selected skills:` are transport metadata. The chat UI renders them as cards instead of exposing the raw marker text.
- Long attachment names truncate inside the card; the full value remains available through the control's title.
- Remove controls live inside the related card and retain an accessible label.

## Topbar and inspector chrome

- The conversation topbar keeps session tabs as the primary signal. Inbox, terminal, and inspector toggles live in `.topbar-actions`.
- Status text appears only when non-empty (or when an API-key action is required) and truncates with a `title` for the full value.
- Specialist labels stay quiet text, not status pills.
- Artifact type badges are neutral mono labels; only tabular data keeps a clay accent. Prefer `--ok` / `--err` / `--clay` over one-off HSL pill colors.

## Responsive workspace layout

- The default 1100 px desktop window keeps the sidebar, conversation, and Inspector as resizable columns. The Inspector becomes a modal drawer only below 960 px, where preserving the conversation width takes priority.
- Scrollable lists keep stable scrollbar gutters and contain overscroll so a nested list does not unexpectedly move the surrounding workspace.

## Dense settings lists

- Long capability lists expose status filters and a visible/enabled count before the rows.
- Secondary editors such as skill tags stay collapsed until requested; the row keeps a short summary so existing metadata remains discoverable.
- Settings that save on interaction say so explicitly, including when changes apply only to new sessions. Empty filter results show an explanatory state instead of a blank list.
