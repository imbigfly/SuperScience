---
name: handwriting-extract
description: >-
  Extract handwritten lab notes, CRF pages, or whiteboard tables from photos
  into a CSV, then calibrate flagged cells with the bound calibration model.
  Use for 手写数据提取, handwritten table OCR, CRF photo to spreadsheet.
  Not a data-cleaning or statistics skill.
---

# Handwritten data extract

Turn photos of handwritten tables into a project CSV. Do not invent numbers.
This skill is owned by the Handwriting Extract specialist. Analysis pixels go
to the card's image-analysis model (`view_image`). Calibration pixels go only
through `calibrate_handwriting`.

## Intake (at most five questions)

1. Ask the user to **upload or paste handwritten photos in this chat**
   (png/jpg/webp). A folder path is OK only if they name it themselves.
2. Optional: expected columns or units if the header is missing.

Hard intake rules:

- Do not list, glob, find, or browse the project looking for images.
- Do not call `view_image`, write extract JSON, or start calibration until
  this chat has user attachments or an explicit user-named path.
- Pre-existing files under `uploads/`, `data/`, or elsewhere are off-limits
  until the user points at them.
- Do not ask row/column counts you can read from the images.

## Privacy

If no image-analysis or calibration model is configured, stop and point to
the capability card settings. Do not pretend to read the photos.
Do not mention `view_image`, `calibrate_handwriting`, or the outbound text
firewall in user-facing replies.

## Workflow

1. For each image, `view_image` and extract JSON only:

```json
{
  "batch": "lab-2026-08-29",
  "pages": [
    {
      "page": 1,
      "image": "uploads/page1.jpg",
      "headers": ["col"],
      "rows": [
        {
          "cells": [
            {
              "text": "",
              "confidence": 0.0,
              "uncertain": false,
              "reason": "",
              "bbox": [0.1, 0.2, 0.08, 0.04]
            }
          ]
        }
      ]
    }
  ]
}
```

`bbox` is optional, normalized `[x, y, w, h]` in 0–1 of the source image.
Omit it when you cannot locate the cell; calibration will then second-look
the whole page plus row/column hints.

2. Align pages to one schema (first page or user-specified headers). Extra
   columns become `extra_*`. Missing cells stay empty with `missing`.
3. Write `data/extracted/<batch>.json`.
4. You MUST call `calibrate_handwriting` with that path. Do not apply your
   own confidence heuristics as a substitute. The tool:

   - flags cells by rules (confidence, totals, date order, units, ranges)
   - second-looks **only** flagged cells with the calibration model
   - writes CSV, `qa.json`, reliability, and annotated images

5. Reply with the tool result: CSV path, reliability (`ok_ratio`,
   `mean_confidence`, uncertain/conflict counts — call this 可信度评估 /
   reliability, never medical accuracy), the QA list, annotated image paths,
   and whether to recapture or hand-edit.

## Hard rules

- Do not send the user to `data_cleaning` until a flagged CSV exists.
- Do not use Tesseract book-OCR scripts from other skills.
- Do not claim medical accuracy. Humans confirm uncertain cells.
- Do not call `view_image` again to "calibrate". That still hits the
  analysis model. Second looks belong in `calibrate_handwriting`.
