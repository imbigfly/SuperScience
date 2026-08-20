---
name: handwriting-extract
description: >-
  Extract handwritten lab notes, CRF pages, or whiteboard tables from photos
  into a CSV, then calibrate and flag uncertain cells. Use for 手写数据提取,
  handwritten table OCR, CRF photo to spreadsheet. Not a data-cleaning or
  statistics skill.
---

# Handwritten data extract

Turn photos of handwritten tables into a project CSV. Do not invent numbers.

## Intake (at most five questions)

1. Ask for image attachments or a folder of png/jpg/webp.
2. Optional: expected columns or units if the header is missing.

Do not ask row/column counts you can read from the images.

## Privacy

`view_image` sends **pixels**. The outbound text firewall does not cover
handwriting in photos. Warn once: redact or crop identifiers first, or use a
local vision endpoint.

If no vision model is configured, stop and point to Settings → Models
(image analysis). Do not pretend to read the photos.

## Workflow

1. For each image, `view_image` and extract JSON only:

```json
{
  "page": 1,
  "headers": ["col"],
  "rows": [
    {
      "cells": [
        {"text": "", "confidence": 0.0, "uncertain": false, "reason": ""}
      ]
    }
  ]
}
```

2. Align pages to one schema (first page or user-specified headers). Extra
   columns become `extra_*`. Missing cells stay empty with `missing`.
3. Write `data/extracted/<batch>.csv` and `data/extracted/<batch>.qa.json`.
4. Calibrate (do not re-OCR):
   - confidence < 0.7 → uncertain
   - row totals, date order, unit clashes, implausible ranges → conflict
5. Add a `_flag` column: `ok` / `uncertain` / `conflict`.
6. Reply with the CSV path, a short QA list (page / cell / reason), and
   whether to recapture or hand-edit.

## Hard rules

- Do not send the user to `data_cleaning` until a flagged CSV exists.
- Do not use Tesseract book-OCR scripts from other skills.
- Do not claim medical accuracy. Humans confirm uncertain cells.
