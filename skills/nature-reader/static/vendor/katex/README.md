# KaTeX (offline)

Local copy of KaTeX 0.16.21 used by `scripts/build_reader_html.py`.

Do not replace these files with a CDN URL. The HTML builder inlines
`katex.css` (without `@font-face`) and `katex.js` so the report works
offline under `file://` and in the app preview.
