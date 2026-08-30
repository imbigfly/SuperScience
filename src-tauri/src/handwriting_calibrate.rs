//! `calibrate_handwriting`: rule-flag extracted tables, then second-look only
//! the flagged cells with the handwriting specialist's bound calibration model.

use async_trait::async_trait;
use base64::Engine;
use image::{GenericImageView, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use superscience_llm::{Content, ImageUrl, Message, Part, ToolSchema};
use superscience_store::Store;
use superscience_tools::{Tool, ToolEnv, ToolResult};

pub const TOOL_NAME: &str = "calibrate_handwriting";
const CONFIDENCE_CUTOFF: f64 = 0.7;
const CROP_PAD: f64 = 0.15;
const UNCERTAIN: &str = "uncertain";
const CONFLICT: &str = "conflict";
const OK: &str = "ok";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractBatch {
    #[serde(default)]
    pub batch: String,
    #[serde(default)]
    pub pages: Vec<ExtractPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractPage {
    #[serde(default)]
    pub page: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub rows: Vec<ExtractRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExtractRow {
    #[serde(default)]
    pub cells: Vec<ExtractCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractCell {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub uncertain: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reliability {
    pub ok_ratio: f64,
    pub mean_confidence: f64,
    pub uncertain_n: usize,
    pub conflict_n: usize,
    pub cell_n: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellReviewRequest {
    pub page: u32,
    pub row: usize,
    pub column: String,
    pub text: String,
    pub flag: String,
    pub reason: String,
    pub confidence: f64,
    pub has_crop: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellReview {
    Confirm {
        confidence: f64,
    },
    Revise {
        text: String,
        confidence: f64,
    },
    Keep {
        flag: String,
        reason: String,
        confidence: f64,
    },
}

#[async_trait]
pub trait CellReviewer: Send + Sync {
    async fn review(
        &self,
        request: &CellReviewRequest,
        image_data_url: Option<&str>,
    ) -> Result<CellReview, String>;
}

impl ExtractCell {
    fn flag(&self) -> &str {
        self.flag.as_deref().unwrap_or(OK)
    }
}

pub fn parse_extract_batch(value: &Value, fallback_batch: &str) -> Result<ExtractBatch, String> {
    if value.get("pages").is_some() {
        let mut batch: ExtractBatch = serde_json::from_value(value.clone())
            .map_err(|e| format!("invalid extract JSON: {e}"))?;
        if batch.batch.trim().is_empty() {
            batch.batch = fallback_batch.to_string();
        }
        normalize_pages(&mut batch);
        return Ok(batch);
    }
    if value.is_array() {
        let pages: Vec<ExtractPage> = serde_json::from_value(value.clone())
            .map_err(|e| format!("invalid extract page list: {e}"))?;
        let mut batch = ExtractBatch {
            batch: fallback_batch.to_string(),
            pages,
        };
        normalize_pages(&mut batch);
        return Ok(batch);
    }
    if value.get("headers").is_some() || value.get("rows").is_some() {
        let page: ExtractPage = serde_json::from_value(value.clone())
            .map_err(|e| format!("invalid extract page: {e}"))?;
        let mut batch = ExtractBatch {
            batch: fallback_batch.to_string(),
            pages: vec![page],
        };
        normalize_pages(&mut batch);
        return Ok(batch);
    }
    Err("extract JSON must be {pages:[...]}, a page object, or a page array".into())
}

fn normalize_pages(batch: &mut ExtractBatch) {
    for (i, page) in batch.pages.iter_mut().enumerate() {
        if page.page == 0 {
            page.page = (i as u32) + 1;
        }
    }
}

pub fn apply_rule_calibration(batch: &mut ExtractBatch) {
    for page in &mut batch.pages {
        apply_cell_confidence_and_missing(page);
        apply_row_totals(page);
        apply_date_order(page);
        apply_unit_clashes(page);
        apply_implausible_ranges(page);
    }
}

fn mark(cell: &mut ExtractCell, flag: &str, reason: &str) {
    let current = cell.flag().to_string();
    let next = worse_flag(&current, flag).to_string();
    if next != current || cell.reason.is_empty() {
        if cell.reason.is_empty() {
            cell.reason = reason.to_string();
        } else if !cell.reason.contains(reason) {
            cell.reason = format!("{}; {reason}", cell.reason);
        }
        cell.flag = Some(next.clone());
    }
    if next != OK {
        cell.uncertain = true;
    }
}

fn worse_flag<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a == CONFLICT || b == CONFLICT {
        CONFLICT
    } else if a == UNCERTAIN || b == UNCERTAIN {
        UNCERTAIN
    } else {
        OK
    }
}

fn apply_cell_confidence_and_missing(page: &mut ExtractPage) {
    for row in &mut page.rows {
        for cell in &mut row.cells {
            if cell.flag.is_none() {
                cell.flag = Some(OK.into());
            }
            if cell.text.trim().is_empty() {
                mark(cell, UNCERTAIN, "missing");
            } else if cell.uncertain {
                mark(cell, UNCERTAIN, "extractor marked uncertain");
            } else if cell.confidence > 0.0 && cell.confidence < CONFIDENCE_CUTOFF {
                mark(cell, UNCERTAIN, "confidence below 0.7");
            }
        }
    }
}

fn header_is_total(header: &str) -> bool {
    let h = header.to_ascii_lowercase();
    ["total", "sum", "合计", "小计", "总计"]
        .iter()
        .any(|needle| h.contains(needle))
}

fn header_is_date(header: &str) -> bool {
    let h = header.to_ascii_lowercase();
    ["date", "日期", "时间"]
        .iter()
        .any(|needle| h.contains(needle))
}

fn header_is_age(header: &str) -> bool {
    let h = header.to_ascii_lowercase();
    h.contains("age") || h.contains("年龄")
}

fn header_is_percent(header: &str) -> bool {
    let h = header.to_ascii_lowercase();
    h.contains('%') || h.contains("percent") || h.contains("率")
}

fn header_is_count(header: &str) -> bool {
    let h = header.to_ascii_lowercase();
    ["n", "count", "例数", "人数", "次数"]
        .iter()
        .any(|needle| h == *needle || h.contains(needle))
}

fn parse_number(text: &str) -> Option<f64> {
    let trimmed = text.trim().replace(',', "");
    let cleaned = trimmed
        .trim_end_matches('%')
        .split_whitespace()
        .next()
        .unwrap_or("");
    cleaned.parse::<f64>().ok()
}

fn apply_row_totals(page: &mut ExtractPage) {
    let Some(total_idx) = page.headers.iter().position(|h| header_is_total(h)) else {
        return;
    };
    for row in &mut page.rows {
        if row.cells.len() <= total_idx {
            continue;
        }
        let mut sum = 0.0;
        let mut counted = 0usize;
        for (i, cell) in row.cells.iter().enumerate() {
            if i == total_idx {
                continue;
            }
            if let Some(n) = parse_number(&cell.text) {
                sum += n;
                counted += 1;
            }
        }
        let Some(reported) = row
            .cells
            .get(total_idx)
            .and_then(|cell| parse_number(&cell.text))
        else {
            continue;
        };
        if counted > 0 && (reported - sum).abs() > 0.51 {
            mark(
                &mut row.cells[total_idx],
                CONFLICT,
                "row total does not match numeric siblings",
            );
        }
    }
}

fn apply_date_order(page: &mut ExtractPage) {
    for (col, header) in page.headers.iter().enumerate() {
        if !header_is_date(header) {
            continue;
        }
        let mut prev: Option<i32> = None;
        for row in &mut page.rows {
            let Some(cell) = row.cells.get_mut(col) else {
                continue;
            };
            let Some(ord) = parse_date_ordinal(&cell.text) else {
                continue;
            };
            if let Some(p) = prev {
                if ord < p {
                    mark(cell, CONFLICT, "date order went backwards");
                }
            }
            prev = Some(ord);
        }
    }
}

fn parse_date_ordinal(text: &str) -> Option<i32> {
    let t = text.trim();
    let digits: Vec<i32> = t
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect();
    match digits.as_slice() {
        [y, m, d] if *y >= 1000 => Some(y * 10000 + m * 100 + d),
        [d, m, y] if *y >= 1000 => Some(y * 10000 + m * 100 + d),
        _ => None,
    }
}

fn unit_token(text: &str) -> Option<String> {
    let lower = text.trim().to_ascii_lowercase();
    for unit in [
        "mg/dl", "mmol/l", "mg", "ml", "kg", "cm", "mm", "%", "μl", "ul",
    ] {
        if lower.contains(unit) {
            return Some(unit.to_string());
        }
    }
    None
}

fn apply_unit_clashes(page: &mut ExtractPage) {
    for col in 0..page.headers.len() {
        let units: Vec<(usize, String)> = page
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| {
                row.cells
                    .get(col)
                    .and_then(|cell| unit_token(&cell.text))
                    .map(|unit| (i, unit))
            })
            .collect();
        if units.len() < 2 {
            continue;
        }
        let first = &units[0].1;
        if units.iter().any(|(_, unit)| unit != first) {
            for (i, _) in &units {
                if let Some(cell) = page.rows[*i].cells.get_mut(col) {
                    mark(cell, CONFLICT, "unit clash in column");
                }
            }
        }
    }
}

fn apply_implausible_ranges(page: &mut ExtractPage) {
    for (col, header) in page.headers.iter().enumerate() {
        for row in &mut page.rows {
            let Some(cell) = row.cells.get_mut(col) else {
                continue;
            };
            let Some(n) = parse_number(&cell.text) else {
                continue;
            };
            if header_is_age(header) && (n < 0.0 || n > 150.0) {
                mark(cell, CONFLICT, "implausible age");
            } else if header_is_percent(header) && (n < 0.0 || n > 100.0) {
                mark(cell, CONFLICT, "implausible percent");
            } else if header_is_count(header) && n < 0.0 {
                mark(cell, CONFLICT, "negative count");
            }
        }
    }
}

pub fn reliability(batch: &ExtractBatch) -> Reliability {
    let mut cell_n = 0usize;
    let mut uncertain_n = 0usize;
    let mut conflict_n = 0usize;
    let mut conf_sum = 0.0;
    for page in &batch.pages {
        for row in &page.rows {
            for cell in &row.cells {
                cell_n += 1;
                conf_sum += cell.confidence;
                match cell.flag() {
                    UNCERTAIN => uncertain_n += 1,
                    CONFLICT => conflict_n += 1,
                    _ => {}
                }
            }
        }
    }
    let ok_n = cell_n.saturating_sub(uncertain_n + conflict_n);
    Reliability {
        ok_ratio: if cell_n == 0 {
            1.0
        } else {
            ok_n as f64 / cell_n as f64
        },
        mean_confidence: if cell_n == 0 {
            0.0
        } else {
            conf_sum / cell_n as f64
        },
        uncertain_n,
        conflict_n,
        cell_n,
    }
}

pub fn flagged_cells(batch: &ExtractBatch) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for (p, page) in batch.pages.iter().enumerate() {
        for (r, row) in page.rows.iter().enumerate() {
            for (c, cell) in row.cells.iter().enumerate() {
                if cell.flag() != OK {
                    out.push((p, r, c));
                }
            }
        }
    }
    out
}

pub async fn review_flagged_cells<R: CellReviewer>(
    batch: &mut ExtractBatch,
    reviewer: &R,
    image_data_url: impl Fn(usize, &ExtractCell) -> Option<String>,
) -> Result<usize, String> {
    let coords = flagged_cells(batch);
    let mut reviewed = 0usize;
    for (p, r, c) in coords {
        let (request, data_url) = {
            let page = &batch.pages[p];
            let cell = &page.rows[r].cells[c];
            let column = page
                .headers
                .get(c)
                .cloned()
                .unwrap_or_else(|| format!("col_{c}"));
            let data_url = image_data_url(p, cell);
            (
                CellReviewRequest {
                    page: page.page,
                    row: r + 1,
                    column,
                    text: cell.text.clone(),
                    flag: cell.flag().to_string(),
                    reason: cell.reason.clone(),
                    confidence: cell.confidence,
                    has_crop: data_url.is_some(),
                },
                data_url,
            )
        };
        let decision = reviewer
            .review(&request, data_url.as_deref())
            .await
            .map_err(|e| {
                format!(
                    "calibration model failed on page {} {}: {e}",
                    request.page, request.column
                )
            })?;
        let cell = &mut batch.pages[p].rows[r].cells[c];
        match decision {
            CellReview::Confirm { confidence } => {
                cell.flag = Some(OK.into());
                cell.uncertain = false;
                cell.confidence = confidence;
                cell.reason = "calibration confirmed".into();
            }
            CellReview::Revise { text, confidence } => {
                cell.text = text;
                cell.flag = Some(OK.into());
                cell.uncertain = false;
                cell.confidence = confidence;
                cell.reason = "calibration revised".into();
            }
            CellReview::Keep {
                flag,
                reason,
                confidence,
            } => {
                cell.flag = Some(flag);
                cell.uncertain = true;
                cell.reason = reason;
                cell.confidence = confidence;
            }
        }
        reviewed += 1;
    }
    Ok(reviewed)
}

pub fn to_csv(batch: &ExtractBatch) -> String {
    let headers = batch
        .pages
        .first()
        .map(|page| page.headers.clone())
        .unwrap_or_default();
    let mut lines = Vec::new();
    let mut head = headers.clone();
    head.push("_flag".into());
    head.push("_confidence".into());
    lines.push(
        head.into_iter()
            .map(|h| csv_escape(&h))
            .collect::<Vec<_>>()
            .join(","),
    );
    for page in &batch.pages {
        for row in &page.rows {
            let mut flags = Vec::new();
            let mut min_conf = 1.0_f64;
            let mut cols: Vec<String> = headers
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let cell = row.cells.get(i);
                    if let Some(cell) = cell {
                        flags.push(cell.flag().to_string());
                        min_conf = min_conf.min(cell.confidence);
                        csv_escape(&cell.text)
                    } else {
                        String::new()
                    }
                })
                .collect();
            let row_flag = flags.iter().fold(OK.to_string(), |acc, flag| {
                worse_flag(&acc, flag).to_string()
            });
            cols.push(row_flag);
            cols.push(format!("{min_conf:.2}"));
            lines.push(cols.join(","));
        }
    }
    lines.join("\n") + "\n"
}

pub fn qa_items(batch: &ExtractBatch) -> Vec<Value> {
    let mut items = Vec::new();
    for page in &batch.pages {
        for (r, row) in page.rows.iter().enumerate() {
            for (c, cell) in row.cells.iter().enumerate() {
                if cell.flag() == OK {
                    continue;
                }
                let column = page
                    .headers
                    .get(c)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{c}"));
                items.push(json!({
                    "page": page.page,
                    "row": r + 1,
                    "column": column,
                    "text": cell.text,
                    "flag": cell.flag(),
                    "reason": cell.reason,
                    "confidence": cell.confidence,
                    "bbox": cell.bbox,
                }));
            }
        }
    }
    items
}

fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn crop_data_url(image_path: &Path, bbox: [f64; 4]) -> Result<String, String> {
    let img = image::open(image_path)
        .map_err(|e| format!("cannot open {}: {e}", image_path.display()))?;
    let (w, h) = img.dimensions();
    let [x, y, bw, bh] = bbox;
    let pad_x = bw * CROP_PAD;
    let pad_y = bh * CROP_PAD;
    let x0 = (((x - pad_x).max(0.0)) * w as f64).floor() as u32;
    let y0 = (((y - pad_y).max(0.0)) * h as f64).floor() as u32;
    let x1 = (((x + bw + pad_x).min(1.0)) * w as f64).ceil() as u32;
    let y1 = (((y + bh + pad_y).min(1.0)) * h as f64).ceil() as u32;
    let cw = x1.saturating_sub(x0).max(1);
    let ch = y1.saturating_sub(y0).max(1);
    let cropped = img.crop_imm(
        x0.min(w.saturating_sub(1)),
        y0.min(h.saturating_sub(1)),
        cw.min(w),
        ch.min(h),
    );
    encode_png_data_url(&cropped)
}

pub fn page_data_url(image_path: &Path) -> Result<String, String> {
    let img = image::open(image_path)
        .map_err(|e| format!("cannot open {}: {e}", image_path.display()))?;
    encode_png_data_url(&img)
}

fn encode_png_data_url(img: &image::DynamicImage) -> Result<String, String> {
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| format!("cannot encode crop: {e}"))?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(buf)
    ))
}

pub fn annotate_page_image(
    image_path: &Path,
    page: &ExtractPage,
    dest: &Path,
) -> Result<bool, String> {
    let cells: Vec<&ExtractCell> = page
        .rows
        .iter()
        .flat_map(|row| row.cells.iter())
        .filter(|cell| cell.flag() != OK && cell.bbox.is_some())
        .collect();
    if cells.is_empty() {
        return Ok(false);
    }
    let img = image::open(image_path)
        .map_err(|e| format!("cannot open {}: {e}", image_path.display()))?;
    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    for cell in cells {
        let Some([x, y, bw, bh]) = cell.bbox else {
            continue;
        };
        let color = if cell.flag() == CONFLICT {
            Rgba([200, 40, 40, 255])
        } else {
            Rgba([220, 150, 20, 255])
        };
        let x0 = ((x.max(0.0)) * w as f64).floor() as i32;
        let y0 = ((y.max(0.0)) * h as f64).floor() as i32;
        let x1 = (((x + bw).min(1.0)) * w as f64).ceil() as i32;
        let y1 = (((y + bh).min(1.0)) * h as f64).ceil() as i32;
        draw_rect(&mut rgba, x0, y0, x1, y1, color, 3);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    rgba.save(dest)
        .map_err(|e| format!("cannot write {}: {e}", dest.display()))?;
    Ok(true)
}

fn draw_rect(
    img: &mut RgbaImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: Rgba<u8>,
    thickness: i32,
) {
    let (w, h) = img.dimensions();
    let x0 = x0.clamp(0, w as i32 - 1);
    let y0 = y0.clamp(0, h as i32 - 1);
    let x1 = x1.clamp(0, w as i32);
    let y1 = y1.clamp(0, h as i32);
    for t in 0..thickness {
        for x in x0..x1 {
            put(img, x, y0 + t, color);
            put(img, x, y1 - 1 - t, color);
        }
        for y in y0..y1 {
            put(img, x0 + t, y, color);
            put(img, x1 - 1 - t, y, color);
        }
    }
}

fn put(img: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    let (w, h) = img.dimensions();
    if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
        img.put_pixel(x as u32, y as u32, color);
    }
}

pub fn parse_cell_review(text: &str) -> Result<CellReview, String> {
    let json = extract_json_object(text)
        .ok_or_else(|| "calibration model did not return JSON".to_string())?;
    let value: Value = serde_json::from_str(&json)
        .map_err(|e| format!("calibration model JSON is invalid: {e}"))?;
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let confidence = value
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.8)
        .clamp(0.0, 1.0);
    match action.as_str() {
        "confirm" => Ok(CellReview::Confirm { confidence }),
        "revise" => {
            let text = value
                .get("text")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "revise action is missing text".to_string())?;
            Ok(CellReview::Revise {
                text: text.to_string(),
                confidence,
            })
        }
        "keep" | "uncertain" | "conflict" => Ok(CellReview::Keep {
            flag: if action == CONFLICT {
                CONFLICT
            } else {
                UNCERTAIN
            }
            .into(),
            reason: value
                .get("reason")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("calibration kept the flag")
                .to_string(),
            confidence,
        }),
        _ => Err(format!("unknown calibration action '{action}'")),
    }
}

fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(text[start..=end].to_string())
}

pub struct LlmCellReviewer {
    cfg: superscience_llm::ProviderConfig,
}

impl LlmCellReviewer {
    pub fn new(cfg: superscience_llm::ProviderConfig) -> Self {
        Self { cfg }
    }
}

#[async_trait]
impl CellReviewer for LlmCellReviewer {
    async fn review(
        &self,
        request: &CellReviewRequest,
        image_data_url: Option<&str>,
    ) -> Result<CellReview, String> {
        let provider = superscience_llm::build(self.cfg.clone());
        let prompt = format!(
            "Second-look one extracted table cell. Do not re-OCR the whole page.\n\
page: {}\nrow: {}\ncolumn: {}\nextracted_text: {:?}\nflag: {}\nreason: {}\nconfidence: {:.2}\n\
Return JSON only: {{\"action\":\"confirm|revise|keep\",\"text\":\"...\",\"reason\":\"...\",\"confidence\":0.0}}.\n\
confirm = extracted text is correct. revise = replace with the visible value. keep = still uncertain/conflict.",
            request.page, request.row, request.column, request.text, request.flag, request.reason, request.confidence
        );
        let user = if let Some(url) = image_data_url {
            Message {
                role: superscience_llm::Role::User,
                content: Content::Parts(vec![
                    Part::Text {
                        kind: "text".into(),
                        text: prompt,
                    },
                    Part::Image {
                        kind: "image_url".into(),
                        image_url: ImageUrl { url: url.into() },
                    },
                ]),
                tool_calls: vec![],
                tool_call_id: None,
                tool_name: None,
                reasoning: None,
                ts: chrono::Utc::now().timestamp(),
                model_name: None,
            }
        } else {
            Message::user(format!(
                "{prompt}\nNo crop is available; judge from the extracted text and flag reason only."
            ))
        };
        let completion = provider
            .complete(
                &[
                    Message::system(
                        "You calibrate one handwritten table cell. Return JSON only. Never invent a value that is not visible.",
                    ),
                    user,
                ],
                &[],
            )
            .await
            .map_err(|e| e.to_string())?;
        parse_cell_review(&completion.content)
    }
}

pub(crate) struct CalibrateHandwritingTool {
    store: Store,
}

impl CalibrateHandwritingTool {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for CalibrateHandwritingTool {
    fn name(&self) -> &str {
        TOOL_NAME
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            TOOL_NAME,
            "Calibrate a handwriting-extract JSON batch: apply table rules, second-look only flagged cells with the bound calibration model, then write CSV, QA, reliability, and annotated images. Call this after writing data/extracted/<batch>.json. Do not substitute your own heuristics.",
            json!({
                "type": "object",
                "properties": {
                    "extract_path": {
                        "type": "string",
                        "description": "Project-relative path to the extract JSON written after view_image"
                    }
                },
                "required": ["extract_path"]
            }),
        )
    }

    fn preview(&self, args: &Value) -> String {
        args.get("extract_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }

    async fn run(&self, args: &Value, env: &dyn ToolEnv) -> ToolResult {
        let extract_path = match args.get("extract_path").and_then(Value::as_str) {
            Some(path) if !path.trim().is_empty() => path.trim().to_string(),
            _ => {
                return ToolResult::fail("calibrate_handwriting error: 'extract_path' is required")
            }
        };
        let path = match env.resolve_read_path(&extract_path, false) {
            Ok(path) => path,
            Err(error) => return ToolResult::fail(format!("calibrate_handwriting error: {error}")),
        };
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) => {
                return ToolResult::fail(format!(
                    "calibrate_handwriting error: cannot read {}: {error}",
                    path.display()
                ))
            }
        };
        let value: Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(error) => {
                return ToolResult::fail(format!(
                    "calibrate_handwriting error: invalid JSON: {error}"
                ))
            }
        };
        let fallback = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("batch")
            .to_string();
        let mut batch = match parse_extract_batch(&value, &fallback) {
            Ok(batch) => batch,
            Err(error) => return ToolResult::fail(format!("calibrate_handwriting error: {error}")),
        };
        apply_rule_calibration(&mut batch);

        let Some(model_id) =
            crate::specialists::handwriting_extract_calibration_id(&self.store).await
        else {
            return ToolResult::fail(
                "calibrate_handwriting error: no calibration model. Choose one in the handwriting-extract capability card settings.",
            );
        };
        let cfg = match crate::build_assigned_vision_provider_config(&self.store, &model_id).await {
            Ok(cfg) => cfg,
            Err(error) => return ToolResult::fail(format!("calibrate_handwriting error: {error}")),
        };
        let reviewer = LlmCellReviewer::new(cfg);
        let root = env.project_root().to_path_buf();
        let page_images: Vec<Option<PathBuf>> = batch
            .pages
            .iter()
            .map(|page| {
                page.image.as_deref().and_then(|image| {
                    env.resolve_read_path(image, false).ok().or_else(|| {
                        let joined = root.join(image);
                        joined.exists().then_some(joined)
                    })
                })
            })
            .collect();
        if let Err(error) = review_flagged_cells(&mut batch, &reviewer, |page_idx, cell| {
            page_images.get(page_idx).and_then(|path| {
                path.as_ref().and_then(|path| {
                    if let Some(bbox) = cell.bbox {
                        crop_data_url(path, bbox).ok()
                    } else {
                        page_data_url(path).ok()
                    }
                })
            })
        })
        .await
        {
            return ToolResult::fail(format!("calibrate_handwriting error: {error}"));
        }

        match write_outputs(&root, &mut batch) {
            Ok(summary) => ToolResult::ok(summary.to_string()),
            Err(error) => ToolResult::fail(format!("calibrate_handwriting error: {error}")),
        }
    }
}

fn write_outputs(root: &Path, batch: &mut ExtractBatch) -> Result<Value, String> {
    if batch.batch.trim().is_empty() {
        batch.batch = "batch".into();
    }
    let dir = root.join("data").join("extracted");
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let stem = sanitize_batch_name(&batch.batch);
    let json_path = dir.join(format!("{stem}.calibrated.json"));
    let csv_path = dir.join(format!("{stem}.csv"));
    let qa_path = dir.join(format!("{stem}.qa.json"));
    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(batch).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("cannot write {}: {e}", json_path.display()))?;
    std::fs::write(&csv_path, to_csv(batch))
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;
    let items = qa_items(batch);
    let score = reliability(batch);
    let qa = json!({
        "reliability": score,
        "items": items,
    });
    std::fs::write(
        &qa_path,
        serde_json::to_string_pretty(&qa).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("cannot write {}: {e}", qa_path.display()))?;

    let mut annotated = Vec::new();
    for page in &batch.pages {
        let Some(image) = page.image.as_deref() else {
            continue;
        };
        let src = if Path::new(image).is_absolute() {
            PathBuf::from(image)
        } else {
            root.join(image)
        };
        if !src.exists() {
            continue;
        }
        let dest = dir.join(format!("{stem}.annotated.p{}.png", page.page));
        if annotate_page_image(&src, page, &dest)? {
            annotated.push(rel(&dest, root));
        }
    }

    Ok(json!({
        "csv": rel(&csv_path, root),
        "qa": rel(&qa_path, root),
        "json": rel(&json_path, root),
        "annotated": annotated,
        "reliability": score,
        "qa_list": items,
    }))
}

fn sanitize_batch_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.trim_matches('-').is_empty() {
        "batch".into()
    } else {
        cleaned.trim_matches('-').to_string()
    }
}

fn rel(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn add_configured_handwriting_calibrate_tool(
    agent: &mut superscience_core::Agent,
    store: Store,
    specialist_id: Option<&str>,
) {
    if specialist_id != Some(crate::specialists::HANDWRITING_EXTRACT_ID) {
        return;
    }
    agent.add_tool(Box::new(CalibrateHandwritingTool::new(store)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(text: &str, confidence: f64) -> ExtractCell {
        ExtractCell {
            text: text.into(),
            confidence,
            uncertain: false,
            reason: String::new(),
            bbox: None,
            flag: None,
        }
    }

    fn page(headers: &[&str], rows: Vec<Vec<ExtractCell>>) -> ExtractPage {
        ExtractPage {
            page: 1,
            image: None,
            headers: headers.iter().map(|s| (*s).to_string()).collect(),
            rows: rows.into_iter().map(|cells| ExtractRow { cells }).collect(),
        }
    }

    #[test]
    fn rules_flag_low_confidence_missing_total_date_unit_and_range() {
        let mut batch = ExtractBatch {
            batch: "t".into(),
            pages: vec![page(
                &["date", "a", "total", "age", "unit"],
                vec![
                    vec![
                        cell("2024-01-02", 0.9),
                        cell("1", 0.9),
                        cell("3", 0.9),
                        cell("200", 0.9),
                        cell("10 mg", 0.9),
                    ],
                    vec![
                        cell("2024-01-01", 0.9),
                        cell("2", 0.4),
                        cell("2", 0.9),
                        cell("40", 0.9),
                        cell("3 ml", 0.9),
                    ],
                    vec![
                        cell("", 0.0),
                        cell("1", 0.9),
                        cell("1", 0.9),
                        cell("12", 0.9),
                        cell("4 mg", 0.9),
                    ],
                ],
            )],
        };
        apply_rule_calibration(&mut batch);
        let p = &batch.pages[0];
        assert_eq!(p.rows[1].cells[1].flag(), UNCERTAIN);
        assert_eq!(p.rows[2].cells[0].flag(), UNCERTAIN);
        assert_eq!(p.rows[0].cells[2].flag(), CONFLICT);
        assert_eq!(p.rows[1].cells[0].flag(), CONFLICT);
        assert_eq!(p.rows[0].cells[3].flag(), CONFLICT);
        assert_eq!(p.rows[0].cells[4].flag(), CONFLICT);
        assert_eq!(p.rows[1].cells[4].flag(), CONFLICT);
    }

    #[test]
    fn reliability_and_csv_include_flags() {
        let mut batch = ExtractBatch {
            batch: "t".into(),
            pages: vec![page(
                &["name", "n"],
                vec![
                    vec![cell("A", 0.9), cell("1", 0.9)],
                    vec![cell("B", 0.4), cell("2", 0.9)],
                ],
            )],
        };
        apply_rule_calibration(&mut batch);
        let score = reliability(&batch);
        assert_eq!(score.cell_n, 4);
        assert_eq!(score.uncertain_n, 1);
        assert!((score.ok_ratio - 0.75).abs() < 1e-9);
        let csv = to_csv(&batch);
        assert!(csv.contains("_flag,_confidence"));
        assert!(csv.contains("uncertain"));
    }

    #[test]
    fn parse_accepts_single_page_and_batch() {
        let page = json!({
            "headers": ["x"],
            "rows": [{"cells": [{"text": "1", "confidence": 0.9}]}]
        });
        let batch = parse_extract_batch(&page, "fallback").unwrap();
        assert_eq!(batch.batch, "fallback");
        assert_eq!(batch.pages[0].page, 1);
        let wrapped = json!({"batch": "run1", "pages": [page]});
        assert_eq!(parse_extract_batch(&wrapped, "x").unwrap().batch, "run1");
    }

    #[test]
    fn parse_review_actions() {
        assert!(matches!(
            parse_cell_review("{\"action\":\"confirm\",\"confidence\":0.95}").unwrap(),
            CellReview::Confirm { confidence } if (confidence - 0.95).abs() < 1e-9
        ));
        assert!(matches!(
            parse_cell_review("here {\"action\":\"revise\",\"text\":\"38\",\"confidence\":0.8}").unwrap(),
            CellReview::Revise { text, .. } if text == "38"
        ));
        assert!(matches!(
            parse_cell_review("{\"action\":\"keep\",\"reason\":\"blurry\"}").unwrap(),
            CellReview::Keep { reason, .. } if reason == "blurry"
        ));
    }

    struct ScriptedReviewer(CellReview);

    #[async_trait]
    impl CellReviewer for ScriptedReviewer {
        async fn review(
            &self,
            _request: &CellReviewRequest,
            _image_data_url: Option<&str>,
        ) -> Result<CellReview, String> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn second_look_can_revise_or_keep() {
        let mut batch = ExtractBatch {
            batch: "t".into(),
            pages: vec![page(&["age"], vec![vec![cell("3?", 0.4)]])],
        };
        apply_rule_calibration(&mut batch);
        review_flagged_cells(
            &mut batch,
            &ScriptedReviewer(CellReview::Revise {
                text: "38".into(),
                confidence: 0.92,
            }),
            |_, _| None,
        )
        .await
        .unwrap();
        assert_eq!(batch.pages[0].rows[0].cells[0].text, "38");
        assert_eq!(batch.pages[0].rows[0].cells[0].flag(), OK);

        let mut keep = ExtractBatch {
            batch: "t".into(),
            pages: vec![page(&["age"], vec![vec![cell("??", 0.2)]])],
        };
        apply_rule_calibration(&mut keep);
        review_flagged_cells(
            &mut keep,
            &ScriptedReviewer(CellReview::Keep {
                flag: UNCERTAIN.into(),
                reason: "still unread".into(),
                confidence: 0.2,
            }),
            |_, _| None,
        )
        .await
        .unwrap();
        assert_eq!(keep.pages[0].rows[0].cells[0].flag(), UNCERTAIN);
        assert_eq!(keep.pages[0].rows[0].cells[0].reason, "still unread");
    }

    #[test]
    fn annotate_draws_only_when_bbox_present() {
        let tmp = std::env::temp_dir().join(format!("hw_ann_{}.png", uuid::Uuid::new_v4()));
        let dest = std::env::temp_dir().join(format!("hw_ann_out_{}.png", uuid::Uuid::new_v4()));
        let mut img = RgbaImage::new(20, 20);
        for p in img.pixels_mut() {
            *p = Rgba([255, 255, 255, 255]);
        }
        image::DynamicImage::ImageRgba8(img).save(&tmp).unwrap();
        let mut page = page(&["x"], vec![vec![cell("1", 0.2)]]);
        apply_rule_calibration(&mut ExtractBatch {
            batch: "t".into(),
            pages: vec![page.clone()],
        });
        page.rows[0].cells[0].flag = Some(UNCERTAIN.into());
        assert!(!annotate_page_image(&tmp, &page, &dest).unwrap());
        page.rows[0].cells[0].bbox = Some([0.1, 0.1, 0.4, 0.4]);
        assert!(annotate_page_image(&tmp, &page, &dest).unwrap());
        assert!(dest.exists());
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn qa_lists_only_flagged_cells() {
        let mut batch = ExtractBatch {
            batch: "t".into(),
            pages: vec![page(
                &["a", "b"],
                vec![vec![cell("ok", 0.9), cell("", 0.0)]],
            )],
        };
        apply_rule_calibration(&mut batch);
        let items = qa_items(&batch);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["column"], "b");
        assert_eq!(items[0]["flag"], UNCERTAIN);
    }
}
