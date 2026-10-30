# PDFbull bug report

Version 0.19.3, branch `main`, reviewed 2026-10-29. This is the second pass: it covers the first pass plus the files skipped the first time.

## Method and limits

- `cargo clippy --all-targets`: clean, except one `write_with_newline` lint in `tests/regressions.rs:2227`.
- `cargo test`: every suite passes (123 + 1 + 8 + 57 + 7 + 38 tests).
- Every finding below comes from reading the code. **I did not run the app or reproduce any of them**, except that clippy and the tests ran. Each finding says how sure I am.
- **Read in full or nearly in full:**
  - `src/engine.rs`, `src/organize.rs`, `src/form_scripting.rs`, `src/storage.rs`, `src/ocr.rs`, `src/tiled_render.rs`, `src/read_mode.rs`, `src/main.rs`, `src/logging.rs`, `src/commands.rs` (first 120 lines), `src/platform/windows.rs`, `src/ui_gpui/tabs.rs`
  - `src/image_optimizer.rs` (all non-test code)
  - `src/ui_gpui/canvas.rs` (lines 170-1297)
  - `src/ui_gpui/app_view.rs` (about 80% of its 4,138 lines)
  - `src/pdf_engine.rs`: open, render, annotations load/save, search, merge/split/reorder/rotate/delete, redaction, forms fill, watermark, header/footer, sign, stamp, optimize, encrypt
  - `src/models.rs`: settings, tab and viewport code (lines 436-1072)
- **Skimmed or not read:**
  - `src/ui_gpui/sidebar.rs`: lines 420-560 and 700-1107 not read.
  - `src/ui_gpui/dialogs.rs`: lines 1-380 and 900-1200 not read.
  - `src/ui_gpui/ribbon.rs`: lines 1-540 and 670-893 not read.
  - `src/ui_gpui/welcome.rs`: lines 180-580 not read.
  - `src/ui_gpui/log_console.rs`: only the first 110 lines read.
  - `src/ui_gpui/mod.rs`: only the lines around the single-instance drain.
  - `src/pdf_engine.rs`: text extraction, table detection, OCR, printing, conformance, signature/layer/form-field extraction and the test modules.

Summary: 1 critical, 6 high, 12 medium, about 17 low or latent.

---

## Critical

### C1. The Redact tool never redacts anything

**Confidence: high.** I grepped every caller.

- The ribbon offers a Redact tool with the tooltip "Permanently black out content" (`ribbon.rs:585`). The app builds `AnnotationStyle::Redact` annotations from it (`app_view.rs:3729`) and the canvas draws a near-black box over them (`canvas.rs:761`).
- `save_annotations` skips them on purpose: `pdf_engine.rs:2280`, "those are applied via apply_redactions()".
- `DocumentStore::apply_redactions` (`pdf_engine.rs:3714`) has **no caller** in `src/` or `tests/`. There is no `PdfCommand` for it in `commands.rs` or `engine.rs`.

Result: a user draws a redaction box and presses Ctrl+S. The box is dropped and the saved PDF contains the original text. The user believes sensitive content was removed. This is a disclosure failure, not just a missing feature.

**Fix:** add a `PdfCommand::ApplyRedactions` and a UI action that says it is destructive. Until then, hide the tool, or make Save refuse and warn when Redact annotations exist.

---

## High

### H1. Closing a modified tab discards unsaved annotations without asking

**Confidence: high.**

- `Tab.is_modified` only draws a dot (`tabs.rs:128`). Nothing reads it before a close.
- These paths remove the tab and its annotations with no check:
  - Ctrl+W: `app_view.rs:2975-3011`
  - `CloseTab`: `app_view.rs:3346`
  - `CloseOthers`: `app_view.rs:3392`
  - `CloseToRight`: `app_view.rs:3418`
- I found no window-close or quit hook in `src/`. `main.rs:146` only flushes config. Closing the window with modified tabs loses the edits too.

**Fix:** confirm-before-close for tabs with `is_modified`, and a window-close hook.

### H2. Several settings do nothing, including `auto_save` and session restore

**Confidence: high.** I grepped for every reader of each field.

| Setting | Defined | Who reads it |
|---|---|---|
| `auto_save` (default **true**) | `models.rs:479` | nobody; only toggled and saved |
| `restore_session` (default true) | `models.rs:476` | nobody |
| `remember_last_file` (default true) | `models.rs:477` | nobody |
| `render_quality` | `models.rs:475` | nobody in the UI. `request_render_page` hard-codes `RenderQuality::High` (`app_view.rs:748`) |
| `cache_size`, `default_filter`, `accent_color`, `log_level_filter` | `models.rs` | nobody |

- `storage::save_session` and `storage::load_session` (`storage.rs:346-364`) have no callers outside tests, so tabs are never saved or restored.
- Even if the quality setting were read: `RenderQuality` only goes into the render cache key (`pdf_engine.rs:1809`). It never changes how a page is rendered.
- Combined with H1, a user with "Auto save" switched on still loses their edits.

**Fix:** implement them or remove the toggles.

### H3. External links from a PDF open with no scheme check

**Confidence: high for the code path. Not executed.**

- `follow_link` (`app_view.rs:2547`) passes the link's `/URI` straight to `open::that_detached`. The engine copies `annot.uri` into the link list without filtering (`pdf_engine.rs:1107`, `:1212`).
- On Windows, `open` hands the string to the shell. A crafted PDF can therefore carry `file://`, a UNC path, or an application URI scheme, and the click launches it with no prompt.

**Fix:** allow only `http`, `https` and `mailto`, and ask the user before opening anything.

### H4. "Properties" and "Extract page" work from stale in-memory bytes, which can erase saved annotations

**Confidence: high for the mechanism. Not executed end to end.**

- `update_metadata` (`pdf_engine.rs:2730-2733`) builds its output from `doc.file().data()`, the bytes loaded when the document was opened. `apply_metadata_update` (`app_view.rs:2832-2853`) then writes that over the file on disk.
- Nothing refreshes `documents[doc_id]` after a save. `save_annotations` and `ExportPdf` write the file but do not reopen it (`engine.rs:452-480` only updates the path).
- Sequence: open a PDF, add a highlight, press Ctrl+S (the file now has the highlight), edit Properties and Apply. The file on disk is overwritten with the original bytes plus the new metadata, so the highlight is gone.
- `extract_pages_to_new` (`pdf_engine.rs:2747`) has the same staleness.
- The `doc_metadata` cache is also not updated after the write, so the dialog reopens with the old values.

**Fix:** reload the document in every worker after any write to its file, or build these outputs from the file on disk.

### H5. "Encrypt" uses fixed, published passwords and the app cannot open its own output

**Confidence: high.**

- `apply_security` (`app_view.rs:1988-1995`) always encrypts with user password `"user"` and owner password `"owner"`. The dialog states this (`dialogs.rs:383`). Anyone can open the file, so the encryption gives no confidentiality.
- After encrypting, the app calls `open_pdf_path` on the result (`app_view.rs:2008`). That path always opens with no password (`app_view.rs:387`), and the app has no password prompt: the dialog says decryption "is not implemented yet" (`dialogs.rs:673`, `app_view.rs:3966`). The new tab is a dead tab with only a status message.

**Fix:** ask for the passwords, and add a password prompt on open.

### H6. Form-calculation parser can panic on non-ASCII text

**Confidence: high. Not executed.**

`form_scripting.rs:334`:
```rust
if pos >= 5 && &expr[pos - 5..pos] == "this." { ... }
```
This slices by byte offset. If `pos - 5` falls inside a multi-byte character (a script such as `"₹ getField(...)"` or an accented letter just before `getField`), the slice panics. The script text comes from the PDF, so a crafted or merely non-English document can crash the thread that evaluates it. I did not confirm that every caller is covered by `catch_worker_panic`.

**Fix:** `expr[..pos].ends_with("this.")`.

---

## Medium

### M1. Expression evaluator gets unary minus and parentheses wrong

**Confidence: high by reasoning. Not executed.** `form_scripting.rs:400-459`.

- It scans right to left for `+`/`-` and treats a `-` as unary only at index 0 of the whole string. `2 * -3` splits at the `-` into `"2 *"` and `"3"`; the left part evaluates as `2 * ""` = 0, so the result is `-3`, not `-6`. `10 - -3` gives 7, not 13. Negative field values substituted by `getField(...)` reach this path.
- `eval_single_comparison` (`:376`) trims leading `(` and trailing `)` from the whole comparison. `"(1+2)*(3+4) > 5"` becomes `"1+2)*(3+4) > 5"`.
- `!=` is not supported.
- Recursion depth grows with the number of terms. A long `1+1+1+...` script from an untrusted PDF can overflow the stack, which aborts the process.

**Fix:** replace it with a small precedence-climbing parser with a depth limit.

### M2. Every page uses one page size, so mixed-size documents are stretched and mis-overlaid

**Confidence: high for the mechanism.**

- `viewport.page_width` and `page_height` are single values: the maximum width and the **first page's** height (`app_view.rs:410-437`). `display_size` (`canvas.rs:367`), the card size, the slot pitch, and the overlay and selection maths (`canvas.rs:686`, `app_view.rs:988`, `:3667`) all use them for every page.
- The bitmap is drawn with `.w_full().h_full()` (`canvas.rs:1017`), so a landscape page in a portrait document is stretched to the first page's shape.
- Annotations, search hits and text selections on those pages are placed against the wrong size.
- Before the engine answers, `inspect_pdf_geometry` (`app_view.rs:90`) reads only the first page's own `/MediaBox`. It misses inherited boxes, `/CropBox` and `/UserUnit`, and falls back to A4.

### M3. Annotation tools do not produce what they preview or what their name says

**Confidence: high by reading; not executed.**

- **Underline / Strikeout** (`app_view.rs:3685-3696`) both create a plain `Line`. The drag box is saved as a diagonal line from its top-left to bottom-right corner (`pdf_engine.rs:2386-2402`), not an underline or strike-through.
- **Line direction is lost.** The drag is normalised with `min`/`max` before it is stored (`app_view.rs:3648-3651`), so every line is a top-left to bottom-right diagonal whichever way it was drawn. The code comment at `pdf_engine.rs:2389` says it preserves direction.
- **The canvas preview disagrees with the saved file.** Lines and arrows are painted as a horizontal 2 px bar at the bottom of the box (`canvas.rs:864-877`). Circles are painted as rectangles (`canvas.rs:814-863`).
- **Text and Note tools have no content editor** that I found. Text is created with an empty string, and Note with the fixed comment `"Note"` (`app_view.rs:3723-3749`). An empty FreeText is invisible, and the sidebar lists it with a blank label (`sidebar.rs:633`).
- **The colour picker does nothing for annotations.** `ribbon.active_color` is read only to set the on-screen selection tint (`app_view.rs:3251`). Annotation colours are hard-coded in the match (`#FFF000`, `#2563EB`, ...).

### M4. Stale render results can overwrite a fresh render

**Confidence: medium-high. Not executed.**

- `request_render_page` (`app_view.rs:714-850`) captures `zoom`, `rotation` and the midnight filter when it sends the request. On the reply it checks only the document id, and re-requests only when **zoom** changed (`:816`).
- `rotate_active_pages` (`:2029`) and `ToggleMidnight` clear `rendering_pages` and start new renders while the older ones are still in flight. If the older reply arrives last, a bitmap with the wrong rotation or filter is stored with `rendered_zoom` equal to the current zoom. Nothing re-renders it until the zoom changes.

### M5. Save marks the tab clean even if the user edited during the save

**Confidence: medium.** `save_active_document` (`app_view.rs:1559-1626`) snapshots `viewport.annotations`, awaits the engine, then clears `is_modified`. An annotation added in between is not in the saved file but the unsaved-changes dot disappears.

### M6. Operations that write a user-chosen path can overwrite the open file and leave a stale view

**Confidence: medium-high.**

- Watermark, header/footer, compress, delete-page, encrypt and merge ask for an output path, then call `open_pdf_path(&out_path)`.
- The save dialog can return the file that is already open. `open_pdf_path` finds the existing tab by path (`app_view.rs:307-316`) and just switches to it. The engine's in-memory document is not reloaded, so the view still shows the old content and old page count while the file on disk has changed.
- After "Delete page" over the same file, the on-screen page numbers no longer match the file.
- These operations also read the file on disk, so they ignore any unsaved annotations in the tab.
- **To verify:** whether `reorder_pages` actually drops the deleted page's content from the file. `IncrementalWriter` appends revisions, and the redaction code's own comment says that keeps the original bytes (`pdf_engine.rs:3711`). If it applies here, a "deleted" page stays recoverable inside the file.

### M7. Watermark and header/footer use the page size but not its origin or rotation

**Confidence: high for origin and isolation; medium for rotation.**

- `media_box_size` (`pdf_engine.rs:167`) returns only `(w, h)` and drops `x0`/`y0`. Text is then positioned from `(0, 0)`. On a page whose box does not start at the origin (for example `[0 100 612 892]` or a `CropBox`), the watermark is off-centre and the header or footer can land outside the visible area.
- `/Rotate` is ignored, so on a rotated page the text appears sideways and the header goes to a side edge.
- The new content stream is appended without wrapping the original content in `q ... Q`. If the page leaves a modified graphics state (a scaling `cm`, for example), the added text is drawn under that transform.
- The watermark is solid grey (`0.7`) with no transparency, drawn **over** the page. It covers the text beneath it rather than sitting behind or blending.

### M8. Image optimiser quality problems

**Confidence: medium-high.** `image_optimizer.rs`.

- `resize_nearest` (`:612`) downsamples by picking every n-th pixel. The comment defends it for bilevel scans, but at 300 → 150 dpi thin strokes and fine lines can vanish, and photos alias. The same function feeds the JPEG tier.
- `forms_seen.insert(id)` (`:290`) skips a Form XObject used a **second** time on the same page, although the comment above it says a reused form is still followed. A form drawn small first and large second is judged only by the first placement, so its image can be reduced below the 150 dpi floor at the larger placement.
- Re-encoded images are tagged `DeviceRGB` (`:726`). An `ICCBased` or CMYK source loses its profile and its colours change.

### M9. Page Organizer "Rotate" only turns the view, for the whole document

**Confidence: high.** The dialog (`dialogs.rs:389-438`) offers "Rotate 90° CW/CCW/180°". `rotate_active_pages` (`app_view.rs:2029`) changes `viewport.rotation`, which applies to every page and is not written to the file. Reopening the file shows it unrotated. The engine's `rotate_pages` (page-level, persistent) has no caller.

### M10. High zoom is blurry because the render scale is capped

**Confidence: high.** The zoom range is 0.25-5.0 (`app_view.rs:1167`), but the render scale is `(zoom * 1.5).clamp(0.75, 3.0)` (`:738`). Above zoom 2 the bitmap is stretched, and `rendered_zoom` records the requested zoom, so it is never re-rendered sharper. `tiled_render.rs` was written for deep zoom but nothing uses it.

### M11. The page box silently falls back to Letter size

**Confidence: medium.** `save_annotations` (`pdf_engine.rs:2291-2301`) and `apply_redactions` (`:3740-3750`) substitute 612×792 when the in-memory document or page is missing. Annotations land in the wrong place, and a redaction would burn the wrong region. It should be an error.

### M12. Several file reads are uncapped, and one blocks the UI thread

**Confidence: high.** Every other entry point uses `read_input_capped` (512 MB).

- `save_annotations` (`pdf_engine.rs:2217`) and `apply_redactions` (`:3725`) call `std::fs::read`.
- `load_annotations` (`:1560`) calls `lopdf::Document::load(path)`, which parses the whole file.
- `inspect_pdf_geometry` (`app_view.rs:90`) does the same on the **UI thread** during `open_pdf_path` and tab switches.

---

## Low / latent

- **L1. Encrypted-PDF handling is inconsistent.**
  - The engine supports passwords in `open_document` and `save_annotations`, but `split_pdf` is always called with `None` (`engine.rs:496`) even though the engine holds the password.
  - `fill_form`, `merge_documents`, `reorder_pages`, `rotate_pages` and `delete_pages` use `IncrementalWriter::new` with no password.
  - Today this is latent, because the UI cannot open an encrypted document (H5).
- **L2. `PageOrganizerState` (`organize.rs`) is not used by the app**, only by `tests/regressions.rs:1826`.
  - Blank pages all share the id `usize::MAX`, and rotations are keyed by page id, so rotating one blank page would rotate every blank page.
  - Undo and redo of reorder and delete do not remap `selected_pages`.
- **L3. Dead code and unreachable features.**
  - Engine commands with no sender in `src/ui_gpui`: `ExtractText`, `ExportImage`, `ExportImages`, `FillForm`, `ListPrinters`, `GetAttachmentBytes`, `LinearizePdf`, `ValidateConformance`, `VerifySignatureTrust`, `CreateBlankDocument`, `SignDocumentWithCert`, `ApplyStamp`, `RotatePages`, `DeletePages`, `ExportSvg`.
  - `tag_pdf` and `ocr_document_parallel` have no callers at all.
  - Unused modules or types: `tiled_render.rs`, `read_mode.rs`, `models::DocumentTab` and `CommandPalette`/`CommandAction`, `models::rotate_coords`/`unrotate_coords`.
  - `TabAction::CloseOthers` and `CloseToRight` are handled but nothing emits them (the tab bar has no context menu).
  - `RibbonAction::Flatten` is handled as "Save As" (`app_view.rs:3224`).
  - `WelcomeState::load_recent_files_impl(scan_page_counts)` is always called with `false`, so page counts are never shown.
- **L4. OCR result is discarded.** `prompt_ocr_page` (`app_view.rs:2316`) reports only "extracted N lines". The text is not shown, copied or saved. The script is hard-coded to Latin.
- **L5. `split_pdf`** does not create `output_dir` and silently overwrites existing `<name>_page_N.pdf` files.
- **L6. `apply_redactions` default path** is built with a hard-coded `/` (`pdf_engine.rs:3796`), and an empty parent gives `/name_redacted.pdf`. Its output is also unencrypted even if the source was encrypted.
- **L7. Single-instance listener** (`platform/windows.rs:133-139`) reads each connection with `read_line` and no length cap, one connection at a time. A connected client that never sends a newline blocks all later forwards, and a very long line uses unbounded memory. The named pipe is not per-user.
- **L8. Opening a document is repeated work.**
  - `open_document` loads and parses the whole Windows root certificate store on every open (`pdf_engine.rs:1020`), before checking whether the file has signatures.
  - Each engine worker that lazily reopens a document (`reload_if_needed`) repeats outline, link, signature and trust work.
- **L9. `open_pdf_path` pushes the tab before the open succeeds** (`app_view.rs:348`). A password-protected or corrupt file leaves a dead tab. Tabs are also matched by raw path string, so `C:\a.pdf` and `c:\A.pdf` open twice.
- **L10. `AppSettings` fields without `#[serde(default)]`.** Adding a new field makes an older `settings.json` fail to parse. The fallback (`storage.rs:158-180`) keeps only four fields, so cache size, quality and the toggles silently reset.
- **L11. Selecting text copies to the clipboard on every drag**, including when an annotation tool is active (`extract_selected_text`, `app_view.rs:1007`).
- **L12. `TileCache::put` "evicts the oldest" entry but picks an arbitrary `HashMap` key** (`tiled_render.rs:130`). Dead code today.
- **L13. Release builds use `windows_subsystem = "windows"`**, so `--help` and `--version` print nothing there.
- **L14. Log console details.** Timestamps are UTC, not local (`logging.rs:132`). Quoted messages are only partly un-escaped (`:98`). `LogLevelFilter::Trace` and `All` behave the same.
- **L15. Search is explicit (button or Enter), but two quick searches can finish out of order**, and the older result can replace the newer one (`app_view.rs:1768-1832`).
- **L16. Tab bar has no horizontal scroll** (`tabs.rs:169`, `overflow_x_hidden`), so tabs past the window width cannot be reached.
- **L17. A second `fill_form`-style hazard to note:** `fill_form`'s combo-box branch writes an empty string when nothing is selected (`pdf_engine.rs:4014`), which clears an existing value.

---

## What looks solid

- Atomic writes (`atomic_write_bytes`) with fsync, retry on sharing violations and no truncating fallback.
- Worker panics are contained with `catch_worker_panic`, including `reload_if_needed`.
- The Windows certificate-store loop avoids a double free and releases the final context.
- The render cache has generation checks so stale renders are not cached.
- The virtualised canvas strip and thumbnail maths are written and documented carefully.
- Many earlier bugs are documented inline and covered by regression tests.

## Suggested order

1. C1: remove or finish the Redact tool.
2. H3: restrict link schemes.
3. H4 and M6: reload documents after writes; handle "output = open file".
4. H1 and H2: close prompts; make or remove the dead settings.
5. H5: real password handling for Encrypt and for opening encrypted files.
6. H6 and M1: rewrite the expression evaluator.
7. M2 to M5, then M7 to M12, then the low items.
