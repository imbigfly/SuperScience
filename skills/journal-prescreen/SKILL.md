---
name: journal-prescreen
description: >-
  Pre-submission author-guideline check for a named journal. Use when the user
  wants 论文预审, 投稿须知, journal checklist, desk-reject format review, or
  "can I submit to this journal". Marks location, severity, guideline quote,
  and a fix. Not scientific peer review — route quality critique to
  academic-paper-reviewer or nature-reviewer.
---

# Journal prescreen

Check a manuscript against **that journal's author guidelines**. Do not invent
rules. Do not run `academic-paper-reviewer` or `nature-reviewer` unless the user
explicitly asks for scientific peer review after this checklist.

## Intake (at most five questions)

1. Manuscript path or attachment.
2. Target journal name, **or** pasted / uploaded 投稿须知.
3. Article type if still unclear (research / review / letter / case).

Do not ask for word counts you can compute yourself.

## Journal profiles

If `journals/<id>.md` matches the journal, load it and record its access date.
If nothing matches, extract a temporary checklist from the user-supplied
guidelines and label it `adhoc`. Never claim an adhoc list is the official
current policy.

## Checklist axes

- Word / figure / table / reference limits
- Required sections and abstract structure
- Ethics, registration, consent, data availability
- Reporting guideline (CONSORT / STROBE / ARRIVE / PRISMA) when the article type needs it
- Reference style and language
- AI-use disclosure only if the guidelines mention it

For each item: `pass` / `issue` / `insufficient`. Issues need:

- location (section / paragraph / table)
- severity (`blocking` or `advisory`)
- quote or paraphrase from the guideline
- a concrete fix

## Deliverable

Write `prescreen/report.md` in the project:

1. Journal + guideline source + access/adhoc flag
2. Fit overview (compliance, not acceptance prediction)
3. Problem table
4. Optional one-line: "For scientific quality, use 国际期刊审稿 or Nature审稿"

## Hard rules

- Do not fabricate ISSN, IF, or acceptance rates.
- Do not copy reviewer-style Major/Minor science critique here.
- If the manuscript is missing, stop and ask for the file.
