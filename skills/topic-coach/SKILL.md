---
name: topic-coach
description: >-
  Coach a clinician to inventory data and materials, tighten one research
  question, and score 3–5 journal-fit topic candidates. Use for 选题引导,
  PICO from my cohort, "what paper can I write with this dataset". Not grant
  writing and not bear-propose collision search unless the user picks a
  candidate afterward.
---

# Topic coach

Help the user go from "I have some data" to a scored topic shortlist.

## Intake (at most five questions)

First ask the user to upload materials (data dictionary, table header, ethics
letter) or to paste an explicit path. Do not list/glob/browse the project to
invent an inventory before they point at files.

Prefer attachments or paths they name over questionnaires. Then ask only for
remaining blockers among:

1. Target journal or article type
2. Time / language constraints
3. Whether they can still collect data

Once materials are in, inspect files yourself. Do not ask for n or column
names you can read.

## Deliverables (write all three)

### `topic/inventory.md`

- Design (retrospective cohort / case-control / RCT / …)
- n, key variables, follow-up, missingness
- Ethics, consent, registration, samples, imaging, existing analyses, papers
- Constraints: deadline, Chinese/English, target journal, ability to add data

### `topic/question.md`

- One-sentence question
- PICO / PECO draft
- What cannot be answered with current materials

### `topic/candidates.md` (main)

3–5 candidates, each with:

- Title (zh + en)
- Article type vs journal section
- **相关性**: question vs journal scope / department
- **数据**: whether n, variables, follow-up support the design; never invent n
- **资料**: ethics, registration, samples, imaging
- **通过率**: qualitative only — `推荐` / `可做但费力` / `不建议`
  grounded in novelty gap, design fit, hard guideline gates, cost to fill gaps.
  Never output a fake acceptance percentage.
- **下一步**: what to collect, what to search, whether to run
  `bear-propose`, `deep-research`, or `journal-prescreen` **after** they pick

## Hard rules

- Success = scoreboard + one next action, not a grant draft.
- Do not start `nature-proposal-writer` unless asked.
- Do not claim you searched the literature unless you actually did.
