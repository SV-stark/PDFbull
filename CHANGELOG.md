# Changelog

All notable changes to the PDFbull project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.17.0] - 2026-10-07

### Changed
- **Compress now actually compresses scanned and photo-heavy PDFs (`src/image_optimizer.rs`)**: Compressing a multi-megabyte scan used to shave 100–200 KB and leave the file effectively the same size. The cause was not the deflate settings but *which streams were eligible at all*: `rewrite_pdf`'s `max_image_dimension` downsampler only accepts images that are all of `FlateDecode`-or-unfiltered, 8 bits per component, `DeviceRGB`/`DeviceGray`, and free of `/Mask`, `/Decode` and `/DecodeParms`. Every other image was returned untouched — which is precisely the `DCTDecode` (JPEG) and `CCITTFaxDecode`/`JBIG2Decode` output that dominates real scans and photos, so the pass had almost no eligible work to do.

  A new image pass runs before the existing garbage-collection/compression pass, dispatching on the image's existing `/Filter`:
  - **`DCTDecode` and `FlateDecode`** → decode, downscale, re-encode as baseline JPEG at quality 75.
  - **`CCITTFaxDecode` and `JBIG2Decode`** → decode to bilevel, downscale, re-encode as JBIG2 via the new `jbig2enc-rust` dependency. Bilevel text compresses far better under JBIG2's arithmetic coder than any JPEG quality setting, and re-encoding these as grayscale JPEG visibly softens the page.

  Downsampling is driven by *effective DPI* rather than a fixed pixel cap, because a PDF stores no DPI: pixel dimensions come from the image dictionary and the size an image is drawn at comes from the page content stream's `cm` matrix. `image_optimizer` walks each page's operators (`q`/`Q`/`cm`/`Do`, recursing into Form XObjects) to derive `pixel_width / (placed_width_pt / 72)`, then halves it — floored at 150 dpi and never upscaling, so a 1200 dpi scan becomes 600 while a 200 dpi scan becomes 150 rather than 100. An image present in resources but never drawn has no derivable DPI and is left alone rather than resampled on a guess.

  Every step is guarded: a failed decode, an ineligible image, or a re-encode that did not shrink the stream keeps the original bytes, so a failure costs file size and never correctness. Images carrying `/SMask` or `/Mask` are skipped outright, since a replacement stream would silently drop their alpha or stencil. Inline (`BI…ID…EI`) images and `JPXDecode` streams are left untouched.

  Note the structural fix: `rewrite_pdf` renumbers every object, so the image pass must run *against the original parse* and its replacements be applied via `IncrementalWriter` before that pass runs.

### Added
- **`jbig2enc-rust` (JBIG2 encoding)**: MIT / Apache-2.0, pinned to `=0.5.4`, added for the bilevel tier above. Only the `encode` feature is enabled — `zpdf` already decodes JBIG2 on the read path, so the encoder's `decode` half is dead weight in the binary. The lossless (no symbol dictionary) configuration is used deliberately: the encoder documents that symbol dictionaries built by independently-encoded images desync their symbol indices and produce undecodable pages, and sharing one dictionary across a whole document would need all images planned in a single pass.
- **`zpdf-content` (content-stream tokenizer)**: pinned to `=0.14.0`, matching `zpdf` 0.14.0 so both resolve to one copy of the types. `zpdf` re-exports the crate's high-level helpers but not its tokenizer, and the interpreter reports a display-list-local image id rather than the document object id needed to rewrite a stream — so the DPI pass re-walks the placement operators directly.
- **Acrobat-compatible JavaScript Form Scripting Engine (`src/form_scripting.rs`)**:
  - Implemented `PdfFormScriptEngine` to evaluate AcroForm calculations and validations in pure Rust without external C++ or runtime dependencies.
  - Emulates the Acrobat Document Object Model (`this.getField("fieldName").value`, `this.getField("fieldName").name`).
  - Emulates the Acrobat `event` lifecycle object (`event.value`, `event.rc`, `event.target`).
  - Built-in support for standard Adobe Acrobat calculation routines: `AFSimple_Calculate(cFunction, cFields)` covering `"SUM"`, `"PRD"`, `"AVG"`, `"MIN"`, and `"MAX"`.
  - Supports dynamic formula evaluations and multi-field arithmetic (e.g., invoice subtotals, tax calculations, item quantities) with safe division and recursive arithmetic parsing.
  - Supports field validation scripts (`event.rc`) for range boundaries and value constraints.
  - Provides a cascading calculation runner (`recalculate_all`) to automatically evaluate field dependencies across document forms.
- **Interactive "Organize Pages" Card Grid (`src/organize.rs`)**:
  - Implemented `PageOrganizerState` light-table controller with multi-selection support (single click, `Ctrl`/`Cmd` click, `Shift` range click, `Ctrl+A` select all).
  - Drag-and-drop page card reordering (`reorder_card`), multi-page batch rotation (`rotate_selected`), batch deletion (`delete_selected`), and blank page insertion (`insert_blank_page`).
  - Full Undo/Redo history stack (`undo`, `redo`) with `OrganizeCommand` snapshots (`Reorder`, `Rotate`, `Delete`, `InsertBlank`).
- **Two-Up Facing Pages & Distraction-Free "Read Mode" (`src/read_mode.rs`)**:
  - Implemented `ReadModeConfig` with distraction-free toggle (`F10`) to hide chrome (ribbon, tabs, sidebars, status bars) for focused reading.
  - Two-up facing pages layout with support for `BookCoverOffset` (Page 0 standalone cover followed by facing paired spreads) and continuous two-up spreads.
  - Specialized eye-care reading themes: `Default`, `Sepia`, `DarkNight`, `EyeCareGreen`, and `PurePaper`.
- **Deep Tiled Zoom Rendering (`src/tiled_render.rs`)**:
  - Implemented `TiledPageGeometry` partitioning large high-zoom pages (400%–1600%) into 512x512 pixel tiles.
  - Viewport intersection math (`intersecting_tiles`) to compute and rasterize only visible tiles, eliminating OOM crashes and high memory consumption on massive architectural drawings and blueprints.
  - Integrated `TileCache` with bounded capacity eviction to maintain a flat memory footprint at any magnification.

## [0.16.9] - 2026-10-04

Regression release. Two independent defects combined to make the document area look dead: **scrollbars vanished and Continuous mode stopped scrolling entirely.** Verified: `cargo clippy --all-targets` clean, `cargo fmt --check` clean, 164 tests passing (1 new regression test), and a clean `cargo build --release` producing a binary stamped 0.16.9.

### Fixed (Scrolling — Regression)
- **Scrollbars disappeared and Continuous mode stopped scrolling after switching tabs.** `reset_for_document` deliberately leaves `total_pages` alone, since page count is document data rather than per-document *view* state. But the guard that used to re-derive it could never fire again:

  ```rust
  let needs_geometry = self.viewport.total_pages == 0 || self.viewport.page_width <= 0.0;
  ```

  `DocumentViewport::new` seeds `total_pages = 1` and `page_width = 595`, so both terms are false for the entire life of the process once any document has been opened. The viewport therefore reported whichever document was opened **last**. Open a 10-page document, then a 2-page one, then click back to the first tab: `total_pages` is still 2, so the Continuous strip lays out only two pages, that content fits inside the viewport, `max_offset` stays 0 — and a zero `max_offset` means **no scrollbar is drawn** *and* the wheel handler clamps the offset to `0`. Continuous mode could not scroll and the thumbnail panel lost its scrollbar for the same reason.

  Geometry is now cached per `DocumentId` and restored on activation, seeded from `inspect_pdf_geometry` at open and refined by the engine's authoritative numbers when the document finishes loading. A document with no cached entry is inspected once and then remembered, so the expensive whole-file parse still happens at most once per document rather than on every tab switch — the original reason the guard was written. Entries are released on every close path. `current_page` is also clamped when the incoming document has fewer pages than the one being left.

- **The page placeholder carried a never-ending animation.** In 0.16.6 the "Rendering page…" placeholder was given a `Spinner` on the grounds that an indeterminate indicator beats decorative dots. `Spinner` is `Animation::new(speed).repeat()` — an animation that does not stop and therefore requests a frame forever. Continuous mode builds up to 48 page cards (`MAX_CARD_WINDOW`) while only the pages around the cursor are rasterized, so roughly 40 of them showed this placeholder: **~40 perpetual animations in the hot path**. The window never idled, every frame rebuilt all 48 cards, and the app became unresponsive to input — including the wheel, which is why scrolling looked dead rather than merely jittery. The placeholder is static text again.

  This also violates the "keep animation work bounded" performance rule, and it should never have been introduced as a cosmetic touch inside a virtualized list.

### Tests
- A regression test locking in the geometry contract: two documents of different lengths must never share page counts across a switch, and the resulting strip must actually overflow a typical viewport so there is something to scroll. 164 tests passing, up from 163.

## [0.16.8] - 2026-10-04

Scroll-correctness fix for the thumbnail panel. The sidebar's virtualisation and its scroll maths disagreed about how tall a thumbnail card is, by a margin that grew with every page. Verified: `cargo clippy --all-targets` clean, `cargo fmt --check` clean, 163 tests passing (3 new regression tests), and a clean `cargo build --release` producing a binary stamped 0.16.8.

### Fixed (Scrolling)
- **Scrolling the Pages panel ran into blank space, worse the longer the document.** The strip's windowing maths mapped a scroll offset to a page index using a hard-coded `THUMB_STRIDE = 200`, but a card actually measured about **206px**: `p_2` (16) + `border_1` (2) + a 160px image + `pt_1` (4) + a text label whose height came from font metrics + `gap_2` (8). The strip's real content therefore outgrew the assumption by ~6px *per page*.

  That drift was not a rounding error. `thumbnail_window` derives the viewport as `assumed_content - max_offset`, and `max_offset` is computed by GPUI from the **real** content size — so the subtraction silently returned `true_viewport - drift`. On a 500-page document the derived viewport was roughly **3000px too short**: the built card window ended far above the visible area, and scrolling the panel showed empty space where thumbnails should have been. The defect scaled with page count, so it was most visible exactly where virtualisation matters most.

  The container's own `p_2` contributed a further 16px that no spacer accounted for.

  Fixed by making the numbers agree by construction rather than by coincidence, which is the same treatment the canvas strip received in 0.16.5:
  - The card is given an explicit height derived from the stride (`THUMB_CARD_H = THUMB_STRIDE - THUMB_GAP`), and the page number moved into an absolutely-positioned badge so no font-dependent box sits in the card's flow. Padding and typography can no longer move the pitch.
  - The container's vertical padding is encoded in the spacer elements (horizontal only remains on the container), so the strip's measured height is exactly what the maths assumes.
  - `thumbnail_content_height` and `thumbnail_top_in_content` are now the single source of truth, shared by the spacer layout, the windowing maths and `scroll_thumbnails_to`. `scroll_thumbnails_to` also honours the padding now, so "jump to page N" lands on N's card rather than one card's worth of scroll away.
  - `thumbnail_window` inverts `thumbnail_top_in_content` exactly and builds one card beyond the visible edge, so a partially-visible card is not built and then flicker as it scrolls into view.

### Tests
- Three regression tests asserting the invariant that was broken, over page counts 1–2000, three viewport heights and a sweep of every scroll position: the strip's laid-out height (top spacer + built cards + gaps + bottom spacer) equals the height the maths assumes; the page-origin mapping round-trips exactly; and built cards always reach the bottom of the visible band.
- Verified the tests are non-vacuous by reintroducing the original 6px-per-page drift and confirming they fail (a 250-page document already trips the viewport-coverage assertion).

## [0.16.7] - 2026-10-04

Follow-up to 0.16.6, fixing the most severe issue found while reviewing that release: switching tabs destroyed a document's annotations in memory, and the next save then deleted them from the file. Also tightens the one remaining place where a failed PDF write was discarded. Verified: `cargo clippy --all-targets` clean, `cargo fmt --check` clean, 160 tests passing (1 new regression test), and a clean `cargo build --release` producing a binary stamped 0.16.7.

> Note for anyone building this: the release **link** is the memory-sensitive step here and intermittently dies with `link.exe` exit code 1171 ("Releasing the double mapped memory failed") when more than one job runs. `cargo build --release -j 1` succeeds. That is worth pinning in `justfile`'s `build-release` recipe and in CI.

### Fixed (Silent Data Loss)
- **A failed annotation delete was ignored, so a save could duplicate instead of replace.** The stale-annotation loop in `save_annotations` exists to stop annotations multiplying on every save; it discarded the result of each `delete_annotation`, so a failure left the old annotation in place *and* appended the new set — precisely the duplication the loop prevents — while the save reported success. It now fails the save rather than write a file whose annotation set it cannot vouch for.
- **Switching tabs erased saved annotations from the PDF file.** This is the full chain, and every step of it was already correct in isolation:
  1. The user highlights a page in tab A and presses `Ctrl+S`. `save_annotations` writes each annotation with a `/PDFBULL:<id>` `/NM` marker.
  2. The user switches to tab B. `sync_viewport_to_active_tab` calls `reset_for_document`, which clears `viewport.annotations` — the only copy the view keeps.
  3. The user switches back to tab A. Nothing put them back: `LoadAnnotations` was only ever sent from `open_pdf_path`, never from tab re-activation, so the canvas showed no annotations and the Notes panel was empty.
  4. The user presses `Ctrl+S`. `save_active_document` sends the now-empty `viewport.annotations`, and `save_annotations` *deletes every previously written `/PDFBULL:` annotation* before appending the new (empty) set.

  The user's saved highlights were permanently removed from their file, and the status bar reported "Saved as A.pdf.". The view now keeps a per-document annotation cache: the outgoing document's set is stashed before the reset and restored on re-activation, and a document with no cached set is re-read from the file via a new `request_annotations` helper shared with the open path. Restoring also rebuilds the per-page index, so restored annotations are visible rather than present-but-unpainted. Cache entries are released when a document is closed (tab close, `Ctrl+W`, close others, close to the right) so the map does not grow for the life of the session.

### Tests
- Added a regression test exercising the exact stash → reset → restore cycle a tab switch performs, asserting that all annotations return in order *and* that the per-page index is rebuilt. The invisible-restored-annotation case is the dangerous one: those annotations would be present in the list (so still written to the file) while painting nothing, which is precisely what made the original bug hard to notice. 160 tests passing, up from 159.

## [0.16.6] - 2026-10-04

Dependency refresh, a correctness pass over the PDF-writing paths, and a UI pass over the chrome. Several fixes below were silent data loss; several more were features the interface advertised that could not actually work.

Verified: `cargo clippy --all-targets` clean, `cargo fmt --check` clean, and 159 tests passing (9 new regression tests). **Not verified:** `cargo build --release` did not complete on the machine this was prepared on — `link.exe` aborted with exit code 1171 ("Releasing the double mapped memory failed") on a small proc-macro crate, i.e. an out-of-memory condition during linking rather than a compile or type error. `cargo check --all-targets` passes, so no source error is known; the release link should be re-run on a machine with more memory before publishing binaries.

### Dependencies
- **`cargo update`**: refreshed the whole graph, most notably `gpui-kit` 0.6.5 → 0.7.0 (with `gpui-pre` and its 21 sub-crates 0.3.6 → 0.3.7), `notify` 7 → 8, plus ~90 transitive updates; `inotify` and `multiversion` were dropped. `zpdf`/`zpdf-writer` stay pinned at 0.14.0. Note that `Cargo.lock` is git-ignored in this repository, so this applies to local and CI resolution rather than to a committed lockfile.
- The UI work below relies on APIs present in `gpui-kit` 0.7.0 (`Button::selected`, `Button::tooltip`, `Button::icon`, and the component icon bundle).

### Fixed (Silent Data Loss)
- **`Header & Footer` destroyed every page's `/Resources`**: the function assigned a brand-new `/Resources` containing only `{Font: {F1}}` to every page, discarding whatever was already there. Every `/XObject`, `/ExtGState`, `/ColorSpace`, `/Shading` and `/Pattern` entry was dropped, so images, soft masks and shadings on every page became unresolvable — the saved file rendered blank apart from the header and footer, while the UI reported *"Headers and footers added successfully."* It now merges `F1` into the page's existing resources, as the watermark path already did. A page whose `/Resources` is present but unresolvable is now left untouched rather than being handed the font-only dictionary.
- **Watermarking had the same defect on malformed pages**: when a page's `/Resources` was an indirect reference that did not resolve to a dictionary, `merged_res` stayed `None` and the code fell back to the shared font-only dictionary — replacing the page's real resources. It now leaves such pages alone and logs a warning.
- **Watermarks and headers landed off the page**: both used hard-coded origins (`200, 400` for the watermark, `50, 760` and `50, 30` for the header and footer), so on any page that is not A4-sized the stamp was invisible or absurdly off-centre. Both now derive their placement from the page's own `/MediaBox`, and the watermark's font size shrinks to fit long text.
- **`Ctrl+S` could report success while writing a sidecar**: when the engine's path map had no entry for a document, `SaveAnnotations` fell back to a `<name>_annotated.pdf` sidecar and still returned success, so the user's file was untouched and its unsaved-changes marker was cleared. It now fails loudly (`DocumentPathNotFound`) instead.
- **Saving a choice (combo-box) form field wrote the label into `/V`**: a field's `/Opt` pairs an export value with a label per option and the two routinely differ (`"A"` / `"Apple"`). `fill_form` read `options[idx]` — the display label — and wrote it as the field's value, producing a document no viewer considers filled. `FormFieldVariant::ComboBox` now carries `export_values` alongside `options`, and write-back prefers it. Extraction also matches an existing `/V` against the label as well as the export value, since some producers put the label there.

### Fixed (Broken Features)
- **Midnight mode had no effect**: `RenderKey` did not include the render filter, so an inverted re-render produced a cache key identical to the already-cached normal render. The view invalidated its own bitmaps, but the engine served the cached image from memory and the pages looked exactly the same. The filter is now part of the key (thumbnails stay colour-true so they cannot share an entry with a filtered full-page render).
- **The `Header & Footer` dialog could not be edited**: it rendered its two values as static text in a bordered box with no input widget anywhere. Apply always sent the constructor defaults, so a person read *"Header Text: Confidential Document"*, believed they were editing it, and stamped a header they never chose. Both fields are now real `Input`s, seeded from the current values, with the `{page}`/`{pages}` placeholders documented next to them. The footer default also used `Page %PAGE% of %TOTAL%`, tokens `add_header_footer` never substituted; it now uses `{page}` / `{pages}`.
- **Choosing AES-128 then pressing Apply encrypted with AES-256**: the Apply button sent a hard-coded `"AES-256"` regardless of which option was on screen, and the two option buttons applied *and* closed the dialog, so there was no way to change your mind. The options now only select, and Apply commits the selection.
- **`Decrypt` opened a modal with no input**: it fell into a catch-all arm that rendered the literal placeholder text *"Document Password Configuration"*. `Password` and `Settings` now state plainly that the capability is unavailable instead of showing a form that cannot be filled in.
- **Failed commands reported nothing at all**: eleven call sites used `let _ = cmd_tx.send(..)` followed by `if let Ok(Ok(..)) = rx.await`, so when the bounded 128-slot command channel was full or the engine errored, the `Err`/dropped arms did nothing — no status message, no log entry, no dialog. Printing additionally left *"Sending to printer spooler..."* on screen forever. All eleven now report through two shared helpers (`report_engine_unavailable`, `report_engine_error`) that set the status bar and write to the log console. This is the same class of defect the 0.16.5 pass fixed for OCR and Convert; it was never applied to the rest.
- **Splitting could report success having written nothing**: `split_pdf` took its page count from a second, stricter `lopdf` parse and treated a failure as `0 pages`, silently skipping every requested index and returning `Ok(vec![])` — the UI announced *"Successfully split into 0 pages."* Object-stream, xref-stream and encrypted files all fail that stricter parse. The page count now comes from the `zpdf::PdfFile` that was already parsed, out-of-range indices are logged, and a split that produces no files is reported as a problem.
- **`load_annotations` reported "no annotations" for any file `lopdf` cannot parse**, which includes every password-protected PDF. The view keeps the empty list, so a user concluded their saved highlights were gone — and the next save wrote that empty list back over the file. An unreadable file is now an error.
- **`ExportImages` discarded every per-page failure** and returned `Ok`, so a caller could not distinguish "all pages exported" from "none were"; an unwritable directory produced `Ok(vec![])`, i.e. success with nothing exported. Failures are now collected and reported, and a run that wrote nothing returns an error.
- **The `Ink` and `Text` tools silently produced yellow highlights**: neither had a match arm, so both fell through to `Highlight`. `Text` now creates a real text annotation, and `Ink` — which has no freehand implementation anywhere in the engine — is no longer offered in the ribbon.
- **`Tables (.csv)` never wrote a `.csv`**: the button detects tables on the current page and copies the CSV to the clipboard, but was labelled with a file extension inside a group called "Export Documents". It is now *"Tables to clipboard"*.
- **Switching page layout or toggling the cover left pages permanently blank**: neither `SetLayout` nor `ToggleCover` requested renders, and since `render` deliberately no longer drives rendering, the newly exposed page sat on the *"Rendering page..."* placeholder with no way to recover except scrolling.
- **The Page Organizer closed after the first button press**, even though it is a multi-operation modal; rotating page after page required reopening it each time. Rotation is now repeatable, like the watermark preset chips.
- **"Browse files…" opened two file dialogs**: the button is a descendant of the clickable dropzone and neither handler stopped propagation, so one click fired both and spawned two concurrent native modals. This is the same bubble-order hazard already fixed in the tab strip.
- **A document switch desynchronised the search box**: the clear path set `sidebar.search_query`, which is only a *mirror* of the search editor, so the visible box kept the previous document's query while the panel read "Enter search query above" and pressing Search ran an empty query. Both sources are now reset together.
- **`Close to the right` could crash**: `TabAction::CloseToRight` called `tabs.drain((idx + 1)..)` with no bounds check, and `Vec::drain` panics when the range starts past the end of the vector. Neither sibling arm bounds-checked either. The index is now clamped.
- **`Ctrl+W` discarded a failed `Close`**, leaving the parsed document and its render-cache entries resident for the whole session with no user-visible sign. It now logs and reports it, as the tab close button already did.

### Fixed (Memory & Concurrency)
- **The Windows trust-anchor loader leaked the last `CERT_CONTEXT` per store on every call.** It now releases the final cursor after enumeration. (Advancing the cursor was already the release for every intermediate context — `CertEnumCertificatesInStore` frees the context it is handed — so freeing inside the loop would be a double free.)
- **A dead per-document `cache_keys` vector grew without bound**: it was appended to on every cache-miss render and read by nothing. Because the key's scale component is quantised to `0.01` and the UI drives it from zoom, one page can mint hundreds of distinct keys per session. Removed.
- **`invalidate_document` leaked its bookkeeping**: it removed the cached bitmaps but left the document's `RenderKey` set behind, which then accumulated every key ever produced — only cleared when the tab closed. It now drops the set too.
- **The engine forwarder could stall on a bounded send**: `worker_tx` is a `bounded(256)` crossbeam channel and `send` blocks when full. It was called from inside the tokio forwarder task, so a backlog parked a runtime worker thread and — because the forwarder is the only dispatcher — stopped *every* command from reaching *any* worker while the window still accepted input. It now uses `try_send` and hands a full queue to a short-lived blocking thread.

### Fixed (Robustness)
- **`hex_to_rgb` could panic on a non-ASCII colour string**: it tested `hex.len() == 6` (a *byte* count) and then sliced `&hex[0..2]`, which panics with *"byte index 2 is not a char boundary"* for any 6-byte non-ASCII input. It is a `pub` function, so that was a live panic in library code. It now parses from bytes behind an explicit ASCII-hex-digit guard.
- **`slice::from_raw_parts` was called with a possibly-null pointer** when enumerating Windows certificate stores. A null pointer is undefined behaviour even for a zero-length slice, and a truncated store entry can produce one. Now guarded.

### UI
- **Two status bars were stacked at the bottom of the window.** One (page navigation + zoom) was pinned to the bottom of the right-hand column and existed only on the document workspace; the other (page count + zoom + status message) was pinned to the bottom of the window and existed everywhere. Both printed the page number and the zoom, so the same facts appeared twice at once. There is now a single bar, rendered once at the window level, shared by the welcome screen and the document workspace: document facts on the left, the transient status message in a shrinkable middle lane (`min_w_0` + truncate, so a long message can no longer push the page controls off a narrow window), and page/zoom controls anchored right behind a hairline separator. The page readout sits in a fixed-width lane so it does not shift as the page count grows.
- **Rust `Debug` output was leaking into the interface**: both status bars printed `{:?}` of the layout enum, so people read `TwoPageSpread` and `Continuous` as though they were internal identifiers. A `PageLayoutMode::label()` now supplies sentence-case names.
- **Emoji and geometric glyphs replaced with Lucide icons**: the welcome screen (📥 📂 📑 ✍️ 🔒 📄), the tab strip (📄, `×`, `+`), the sidebar expand control (`▶`), the annotation delete control (`×`) and the signature trust badges (`✓ ⚠ ✕`) all used glyphs that render differently per platform, ignore the theme's foreground colour and cannot dim with a row — despite 0.15.3 having claimed a full migration to vector icons. Everything now comes from the component bundle's icon family, and the icon-only controls carry tooltips.
- **Selection state no longer spends the primary emphasis budget**: the page-layout choices, the Cover/Midnight switches, the twelve annotation tools, the watermark preset chips, the Security algorithm options and the sidebar panel tabs all painted the active item with `primary()` — the strongest fill in the window — which is reserved for the one action a decision area commits. They now carry the component's `selected()` state.
- **In-document search reports its pending state honestly**: the button's label was `"Searching..."` (three dots, and no progress indicator) while an internal guard silently swallowed clicks, so a pending search looked clickable but did nothing. It now shows a spinner, `"Searching…"`, and is genuinely disabled while in flight.
- **Interface copy follows the platform rules**: `…` instead of `...` on buttons that open a dialog, sentence case for the recent-documents heading (was `RECENT DOCUMENTS`), `"Saved as X."` / `"Sent to the printer."` instead of ceremony-laden success messages, and a split that produced no files reported as a problem rather than a success.
- **Recent documents are reachable from the keyboard**: each row was a bare clickable `div` with decorative `"Open →"` text, so it could not be tabbed to or activated with Enter. The rows are real `Button`s and the trailing affordance is an icon.
- **Element ids are stable domain identities**: the recent-documents, bookmark, attachment and layer rows were keyed on their list index, and all four lists are rebuilt from scratch on every document switch — so one document's row *n* inherited another's hover and focus state. Recent files key on the path, bookmarks on target and title, attachments and layers on their PDF object id.
- **Attachment sizes are readable**: the sidebar printed raw counts (`"4194304 bytes"`); sizes are now unit-scaled to KB/MB/GB.
- **Signature trust badges use theme tokens**: six hard-coded RGBA literals could not respond to a custom theme and ignored light-mode contrast; they now read `success`/`warning`/`danger` and their foregrounds.
- **Consistent page geometry**: the document column gained `min_w_0()` so it shrinks instead of clipping around long content.

### Tests
- Added 9 regression tests to `tests/regressions.rs` (28 total, up from 19): both resource-preservation tests above, `hex_to_rgb` panic-freedom on multi-byte and malformed input, layout-mode labels not leaking `Debug` names, `invalidate_document` releasing its key bookkeeping *and* keeping filtered renders out of the unfiltered cache entry, combo-box export-value round-trip plus `serde` tolerance of payloads written before the field existed, the Security dialog's default being one it actually offers, and the exact drain arithmetic `CloseToRight` uses across every stale index. Verified the resource tests are not vacuous by re-introducing the original destructive assignment and confirming they fail.
- 159 tests passing, up from 150.

## [0.16.5] - 2026-09-29

Correctness pass over the GPUI presentation layer and the PDF engine. The baseline before this work was already a clean `cargo clippy --all-targets` with 131 passing tests, so every issue below lived in a path the suite did not cover — mostly the render and event layers. Verified after: clippy clean, `cargo fmt --check` clean, 150 tests passing (19 new regression tests), and a clean `cargo build --release`.

### Fixed (Data Loss & Save Correctness)
- **`Ctrl+S` never wrote the opened file**: the engine's `SaveAnnotations` handler passed `None` as the output path, so `save_annotations` fell back to a `<name>_annotated.pdf` sidecar while the status bar reported "Document saved successfully." The handler now resolves the document's real path from the engine's shared path map, and the status message names the file actually written.
- **Repeated saves duplicated every annotation**: `save_annotations` re-read the already-annotated file from disk on each call and appended the current set again, so highlights silently multiplied on every `Ctrl+S`. Each annotation written by the app is now stamped with an `/NM` marker (`PDFBULL:<id>`), and a save strips all previously-marked annotations before appending — making the operation idempotent.
- **Atomic PDF writes**: annotated documents are now written through a temp file plus rename (`storage::atomic_write_bytes`) instead of a truncating `fs::write`, so a failure part-way through can no longer leave a user's PDF truncated and unopenable.
- **Cross-document annotation contamination**: `open_pdf_path` and `sync_viewport_to_active_tab` cleared only the render caches, leaving annotations, the text layer, search hits, cached page origins and the scroll offset from the previous document. Opening B after A showed A's highlights, and `Ctrl+S` on B wrote A's annotations into B's file. All per-document state is now cleared together via `DocumentViewport::reset_for_document()`.
- **`Arrow` annotations lost their head on save**: they were written as plain `/Line` annotations with no `/LE` array, so `load_annotations` could never restore the arrowhead. Arrows now record `/LE [ /None /Arrow ]` and round-trip correctly.
- **Save handlers captured a stale tab index**: `save_active_document` and `save_as_active_document` captured `active_tab_index` before the async round-trip and used it afterwards. If an earlier tab was closed meanwhile, the wrong tab lost its unsaved-changes marker — or, for Save As, had its path and title rewritten to the export target. Both now match on `doc_id`.
- **Engine reload race**: `reload_if_needed` checked `has_document`, released the path-map lock, then reopened, so a concurrent `Close` could land in the gap and leave the document (and its render-cache entries) resident after it was supposed to close. The re-check and the reopen now happen under a single held read lock.

### Fixed (Broken Features)
- **Rotate and Midnight mode blanked every page permanently**: both cleared `rendered_pages` without clearing `rendered_zoom`, and `request_render_page` gates on the zoom map *first*. Every page therefore short-circuited forever and sat on the "Rendering page..." placeholder until the user changed zoom. Both sites now use a shared `invalidate_rendered_pages()`; rotation also invalidates thumbnails, which carry the same rotation.
- **In-document search was unreachable**: the Search sidebar panel rendered a "Search" button but no text input at all, and `SidebarAction::SearchQuery` was never constructed anywhere — so `search_query` was permanently empty and `execute_search` always took its empty-query early return. A real gpui-kit `Input` is now wired through an `InputEvent::Change` subscription.
- **Search could wedge the panel**: `is_searching` was only cleared on success, so any engine error or a saturated command channel left the button stuck on "Searching..." forever. It is now reset on every outcome, with the failure surfaced in the status bar, and results are capped at 5,000 hits.
- **Layer visibility toggles were purely cosmetic**: the sidebar only flipped a local flag. `ToggleLayer` is now dispatched to the engine and the cached bitmaps are dropped so the change actually re-renders.
- **Watermark preset chips closed the dialog and always stamped "CONFIDENTIAL"**: a chip click dispatched the applying action (which cleared `dialogs.active` unconditionally), and `selected_watermark` was never assigned. Chips now dispatch a non-closing `SelectWatermark` action, and the dialog only closes for actions that apply something.
- **Dead "Form Fields" button**: the ribbon button was wired to an empty match arm. It now calls `GetFormFields` and reports the field count and per-field kind/page to the log console and status bar.
- **Decrypt dialog silently did nothing**: `SetPassword` was an empty arm. It now reports that password-protected PDF support is not implemented rather than closing a modal that did nothing.
- **Apply buttons on non-configurable dialogs**: Password, Settings, Signature and Page Organizer each presented an Apply button wired to a no-op arm. Gated behind a new `dialog_has_apply` predicate.

### Fixed (Input Handling & Scrolling)
- **Mouse wheel scrolled at 2× speed, and Ctrl+wheel panned while zooming**: the canvas combined `.overflow_y_scroll()`, `.track_scroll(handle)` and its own `.on_scroll_wheel()`. Because `track_scroll` shares one offset `Rc` with GPUI's built-in `paint_scroll_listener`, the wheel delta was applied once by the framework and again by the app; during Ctrl+wheel zoom the app changed no offset at all, so the built-in handler panned the view. The app is now the single owner of the scroll offset (`overflow_y_hidden` + `track_scroll`, which still applies the offset, clamps it and feeds the scrollbar).
- **Closing a tab corrupted `active_tab_index`**: the tab's close `Button` is nested inside the tab's own clickable `div`. gpui-kit's `Button` only stops click propagation when disabled or loading, and GPUI dispatches bubble-phase listeners in reverse paint order, so the child fired first and the parent then received `SelectTab(idx)` for the just-removed index with no bounds check. Closing any non-first active tab left `active_tab_index == tabs.len()`, which made every downstream `tabs.get(active_idx)` silently fail — the canvas kept showing the closed document, no tab rendered as active, and Save/Print/OCR no-opped. Fixed with `cx.stop_propagation()` on the close handler plus a bounds check on `SelectTab`.
- **One click deleted two annotations**: annotation ids were assigned as `annotations.len() + 1`. Deleting one of three left ids `{1,3}` with `len() == 2`, so the next annotation reused id 3; the sidebar delete button keys on that id and used `retain(|a| a.id != id)`, removing every match. Ids now come from the existing global `next_annotation_id()` counter, and delete removes exactly one entry.
- **Non-PDF files and directories opened unrecoverable blank tabs**: `open_pdf_path` only checked `exists()`, and the drop handler's `|| path.is_file()` made its extension test redundant. A dropped `.png`, or a directory passed on the command line, created a tab that could never render and reported only via a `tracing::error!` the user never sees. All entry points (both drop handlers, the command line, the recent-files list) now share a single `SidebarState::is_openable_pdf` check and surface a status message.
- **Zoom NaN guard**: selection and annotation coordinate mapping divided by raw `viewport.zoom` in one path while two other paths used `.max(0.1)`. `f32::clamp` propagates NaN, so a single bad wheel delta would have blanked coordinates silently. All three sites now validate for finiteness and range.

### Fixed (Async Correctness)
- **Async results applied with no document guard**: `LoadDocumentMeta`, `LoadAnnotations`, `request_text_items` and `execute_search` wrote into view state unconditionally, so a response for a document the user had since switched away from overwrote the sidebar, results, text layer and highlights of whatever was on screen — and a `LoadAnnotations` response silently discarded a highlight drawn while it was in flight. All four now check `active_doc_id`, and `LoadAnnotations` additionally refuses to clobber existing annotations.
- **Stale render responses unblocked the wrong document**: `rendering_pages.remove(&page_idx)` ran *before* the `active_doc_id` check on every success and error path, so a late response from document A cleared document B's in-flight marker for the same page index and caused duplicate renders.
- **Discarded engine sends reported stale status forever**: `let _ = cmd_tx.send(..)` combined with `if let Ok(Ok(..))` left the pre-set "Running OCR on page N…" message in place indefinitely when the bounded 128-slot channel was full or the engine errored. OCR and Convert now always replace the message with either success or a concrete failure.
- **Convert export reported success without writing**: the result of `std::fs::write` was discarded and "Exported to … successfully." was shown regardless, for a non-atomic whole-document write on the UI thread. Export now writes atomically on a blocking thread and only claims success once the bytes are on disk.
- **`Close` send failures leaked documents**: four `try_send(PdfCommand::Close(..))` results were dropped, so a saturated channel left the parsed document and its render-cache entries resident with no user-visible sign. Failures are now logged and surfaced in the status bar.

### Fixed (Release-Build Integrity)
- **`panic = "abort"` disabled the entire error-handling layer in shipped builds**: `catch_unwind` cannot observe a panic under `abort`, so the 30+ `catch_worker_panic` call sites in `engine.rs` and the crash-report panic hook in `main.rs` were dead code in every release and `dist` binary — a single panic inside `zpdf`/`lopdf` took the whole app down with no report. Removed from both profiles, with an explanatory comment in `Cargo.toml`.
- **Password leaked through `Debug`**: `PdfCommand` derived `Debug` while `Open` carries the document password, so any `tracing::debug!("{:?}", cmd)` would have printed the secret. `Debug` is now hand-written and renders the variant name only. The engine's password clone for `open_document` is also held in a `Zeroizing` buffer rather than a plain `String`.
- **Window creation could abort the process**: `ui_gpui::mod` used `.expect("Failed to open PDFbull window")` inside a detached spawn, where a panic is swallowed — and under the previous `abort` profile it would have killed the process. Errors are now logged and reported on stderr.

### Fixed (Memory & Scrolling)
- **~1.5 GB of resident memory for a 2 MB PDF**: `ContentInterpreter::new` starts from `zpdf`'s `ParseLimits::default()`, whose `max_image_cache_bytes` is **1 GiB**, and the engine hands the interpreter a *document-lifetime* `ImageCache` (so scrolling does not re-decode the same stream on every frame). Nothing ever overrode it. A 300 DPI page decodes to roughly 2480x3508x4 = **35 MB** of RGBA out of only a few hundred KB of JBIG2/JPEG, so scrolling through ~30 pages filled a gigabyte of decoded pixels that stayed resident until the tab closed — plus the 512 MB page-render cache on top. Every interpreter site now passes an explicit `with_image_cache_limit` with a **96 MB** per-document budget (`DEFAULT_IMAGE_CACHE_BYTES`), and `open_document`'s `ParseLimits` additionally caps `max_font_cache_bytes` at 32 MB, down from zpdf's 256 MB.
- **Continuous-mode scrolling skipped pages and could not reach the last page**: the virtualized scroll container carried `.py_8()` *and* `.gap_6()` *and* two spacer elements, so the top padding and inter-page gap were counted twice. Every page card therefore rendered 56 px below where `calculate_visible_page_continuous` believed it was, and the strip came out ~112 px taller than the scroll maths assumed — so the page number in the status bar disagreed with the page actually on screen, and scrolling all the way down left the final page permanently short of the viewport. All vertical spacing is now encoded exactly once, in the spacers and per-card slots, and the container carries neither. `continuous_strip_geometry()` and `page_top_in_content()` are the single source of truth shared by the layout and the scroll maths, and a source-level guard test stops the container's padding/gap from being re-applied.

### Performance
- **Canvas and sidebar were O(total_pages) per frame**: every frame built a full page-card element subtree for every page (plus a thumbnail card per page in the sidebar), each deep-cloning that page's `Vec<TextItem>` and running an O(pages × annotations) filter. Both are now virtualized — the canvas window is derived from the live scroll offset with a hard 48-page cap, the thumbnail strip from the strip's own scroll position with a 64-card cap. The caps are unconditional, so the first frame before layout (where `max_offset` is still zero) cannot regress to building the whole document. Height-preserving spacer elements keep the scrollbar accurate.
- **Per-page annotation index**: annotations are now grouped by page (`annotations_by_page`, rebuilt via `reindex_annotations()`), so painting a page is O(annotations on that page) rather than a full scan of every annotation.
- **No more redundant `RwLock` writes in the paint phase**: `page_origins` is now only written when a card's origin actually moves, and is cleared on document switch instead of accumulating stale entries.
- **Full PDF parse moved off the tab-switch path**: `inspect_pdf_geometry` (a whole-file `lopdf::Document::load`) ran on every open *and* every tab switch. It is now only re-derived when the viewport has no authoritative geometry yet.
- **Config writes serialized**: opening N files at once spawned N threads each writing a stale `recent_files.json` snapshot, so entries were lost. The recent-file list is now a single process-wide source of truth with serialized atomic writers, and the one-time config-directory migration runs under the same lock.
- **Log console is no longer O(n) per line**: `Vec::remove(0)` memmoved every surviving `String` once the buffer was full. Replaced with a `VecDeque` and monotonic sequence numbers used as element ids, so ids stay stable across eviction and filtering.

### Changed
- **Element ids are domain-derived, not list indexes**: tab strips, annotation cards, bookmark rows, search results and log entries now key their `ElementId`s on stable identities (tab id, annotation id, hit position, log sequence). Index-based ids shifted whenever a list changed, handing one item's retained hover, focus and keyboard state to a different item.
- **`render` is now side-effect free**: it previously wrote `viewport.current_page` and then called `render_needed_pages`, which mutates state and detaches engine tasks. A render pass can be speculative and discarded, so that both mutated the model outside the event loop and queued engine commands from inside a frame. Page tracking is now driven by the wheel and canvas mouse-move handlers via a new `sync_current_page_from_scroll()`.
- **Search results are bounded** at 5,000 hits, since the sidebar list and the highlight overlay both grow linearly with the hit count.
- **`search()` rejects an empty query** in the engine rather than relying on the caller: `str::find("")` always matches, so the scan loop returned one hit per character of page text.
- **Removed unreachable code**: the never-called `open_sample_doc` (which fabricated a blank 5-page tab), and a dead render-cache probe in `export_page_as_image` whose key (`rotation: 0, quality: Medium`) the app never produced, so it could never hit.
- **Declared `rust-version = "1.90"`** in `Cargo.toml` to match the edition 2024 + let-chains toolchain requirement.

### Tests
- Added `tests/regressions.rs` (19 tests) covering: annotation id uniqueness after delete, full per-document state reset, the `rendered_pages`/`rendered_zoom` pairing invariant, the per-page annotation index, canvas and thumbnail window bounds and scroll tracking (including the pre-layout fallback), stable log sequence ids, `PdfCommand` password redaction, PDF-only input validation, the Apply-button gate, end-to-end save idempotency and Arrow round-trip against `tests/test_document.pdf`, plus the virtualized strip's card positions and content height matching the scroll maths exactly across page counts and zooms, `scroll_to_page` and `calculate_visible_page_continuous` being exact inverses, the decoded-image budget actually rejecting over-limit images and staying bounded while rendering, and the Continuous branch not re-applying container spacing.
- Existing suites updated for the new `SidebarState::new_for_test()` and `LogConsoleState` entry shapes (150 tests passing, up from 131).

## [0.16.4] - 2026-09-22

### Added & Upgraded (zpdf v0.14.0 & X.509 Trust Chain)
- **Upgraded zpdf and zpdf-writer to v0.14.0**:
  - **Precision Rule-Based Table Detection (`detect_tables_with_rules`)**: Enabled border-and-rule line detection sinking vector lines (`RuleLine`) via `ContentInterpreter::with_rule_sink(&mut rules)` for tabular data parsing. Wired "Tables (.csv)" export buttons into the Convert and Tools ribbon tabs with automatic clipboard copy and status toast notification.
  - **Granular CPU Render Diagnostics (`StageStats`)**: Enabled `.with_stage_timing(true)` on `CpuRenderer` during page rendering and PNG export to profile nanosecond timing breakdowns (glyphs, fills/strokes, images, soft masks).
  - **Shared Font Cache (`SharedFonts`)**: Leveraged document-wide `Arc<LoadedFont>` sharing in `PdfDocument::load_page_fonts` to eliminate redundant font re-parsing across pages.
  - **Scoped Blend-Group Compositing**: Integrated bounded transparency compositing for soft masks and blend groups.
- **X.509 Certificate Chain Trust Verification (`zpdf::trust`)**:
  - Integrated Windows System Root (`ROOT`) and Intermediate CA (`CA`) certificate stores using native Win32 CryptoAPI (`CertOpenSystemStoreW`, `CertEnumCertificatesInStore`).
  - Implemented `verify_certificate_chain` for digital signature CMS blobs, classifying chains into `Trusted(cns)`, `Untrusted(msg)`, and `Unsupported(msg)`.
  - Added visual trust badges in signature models and UI:
    - 🟢 **Verified & Trusted Root CA**: Valid cryptographic signature chaining up to a trusted Windows root authority.
    - 🟡 **Valid Signature (Untrusted Root / Self-Signed)**: Cryptographically intact signature whose certificate root is self-signed or not in the Windows trust store.
    - 🔴 **Invalid Signature / Digest Mismatch**: Corrupted byte-range or digest mismatch.
  - Enhanced the Digital Signatures inspection dialog with full signer details, reason, location, validity status, and the complete evaluated X.509 certificate chain hierarchy.

### Fixed (Canvas Scrolling & Viewport Navigation)
- **Canvas Scrolling & Page 1 Lock Resolution**:
  - Fixed viewer lock to page 1 caused by `ScrollHandle::top_item()` returning 0 on non-virtualized `div` containers, which reset `current_page = 0` on every render pass.
  - Implemented dynamic visible page calculation (`calculate_visible_page_continuous`), accurately deriving the active page from the physical vertical scroll offset (`-scroll_handle.offset().y`) divided by page height plus gap.
  - Implemented `scroll_to_page` for exact, immediate jumping across pages without resetting scroll state.
  - Enabled continuous mouse wheel scrolling in `handle_canvas_scroll_wheel` by applying wheel delta to `scroll_handle.offset().y`, clamping within document bounds, and updating `scroll_handle.set_offset`.
  - Resolved Taffy flexbox minimum size expansion by adding `.min_h_0()` to canvas scroll containers and parent flex column/row containers.
  - Synchronized Next/Prev buttons, PageUp/PageDown/arrow keys, and sidebar thumbnail/search/bookmark clicks to navigate smoothly via `scroll_to_page`.

## [0.16.3] - 2026-09-22

### Performance
- **Memory-bounded render pipeline**: UI `rendered_pages` and `rendered_zoom` maps now kept to a ±3 page sliding window around the current page — off-screen pixel buffers are freed automatically every time a new page renders.
- **Text cache eviction**: `text_cache` (used for drag-selection and copy) is evicted beyond a ±10 page window and transparently re-fetched on demand. `search_highlights` (coordinate-only) are never evicted, so search results persist across scrolling.
- **Low-res sidebar thumbnails**: Sidebar now fetches thumbnails via a dedicated `RenderThumbnail` command at 0.25× scale (`RenderQuality::Low`) instead of duplicating the full-resolution canvas image — approximately 72× RAM reduction per thumbnail page.
- **Narrower scroll prefetch window**: Continuous-scroll mode now prefetches ±3 pages (previously ±8), significantly reducing peak working set on large PDFs.
- **Lazy thumbnail loading**: Thumbnails for nearby pages are loaded lazily when the sidebar panel opens or switches to Thumbnails mode.

### Fixed
- Log console startup message now accurately reports the full render pipeline: `zpdf-render-cpu (tiny-skia) → DirectX 11 texture upload`.

## [0.16.2] - 2026-09-22

### Added & Production-Ready Desktop Features
- **Comprehensive Desktop Feature Wiring**:
  - **Save & Save As**: Wired `Ctrl+S` and Ribbon Save action to execute incremental `PdfCommand::SaveAnnotations` and full `PdfCommand::ExportPdf` via asynchronous file dialogs, persisting all annotations back to PDF files.
  - **Print Pipeline**: Wired `Ctrl+P` and Ribbon Print action to invoke `PdfCommand::PrintPdf` with native Windows printer spooling.
  - **Global Desktop Keyboard Shortcuts**: Registered cross-platform hotkeys including `Ctrl+O` (Open), `Ctrl+S` (Save), `Ctrl+P` (Print), `Ctrl+W` (Close Tab), `Ctrl+F` (Open Search), `Ctrl+=` / `Ctrl+-` / `Ctrl+0` (Zoom In/Out/Reset), `PageUp` / `Left` and `PageDown` / `Right` (Page Navigation), and `Esc` (Dismiss Dialogs/Selections).
  - **Interactive Search Engine & In-Page Highlights**: Connected sidebar search bar with query execution via `PdfCommand::Search`, occurrence count tracking, snippet listing, click-to-jump page navigation, and amber translucent search match highlights overlaid directly on the PDF document canvas.
  - **Rich Sidebar Panels**: Implemented live panels for Bookmarks outline navigation (`LoadDocumentMeta`), Annotations list with per-item jump and delete buttons, Attachments inspection with file size formatting, and optional PDF Layers toggles.
  - **Multi-Style Annotation Tools**: Extended canvas drag creation beyond basic highlights to support Underline, Strikeout, Rectangle, Circle, Line, Arrow, StickyNote, and Redact annotations with distinct rendering styles and modified-document state tracking.
  - **Dark / Midnight Inversion Mode**: Connected Midnight mode toggle to `RenderFilter::Inverted` in `request_render_page` with instant cache clearing and re-rendering for high-contrast nighttime reading.
  - **Book Spread View with Standalone Cover**: Updated `TwoPageSpread` layout to support `standalone_cover`, correctly displaying cover page 0 centered as a single page while pairing subsequent pages as true two-page spreads.
  - **PDF Utility Workflows**: Wired interactive modal dialogs and file operations for Merge (multi-PDF picker and merge), Split (directory export), Compress (`PdfCommand::Optimize`), Add Watermark (preset and custom stamps), Add Header & Footer, Document Encryption (AES-128/256), and Page Organizer (90° CW/CCW rotation and page deletion).
  - **Document Conversion**: Integrated export actions for Markdown (`.md`), HTML (`.html`), and Plain Text (`.txt`) via `PdfCommand::ConvertPdf`.
  - **Developer Log Console**: Built a slide-up log drawer with streaming log entries, severity level filtering (`all`, `info`, `warn`, `error`), one-click Copy to Clipboard, and Clear Log capability.
  - **Status Message Banner**: Integrated live operation feedback (save status, encryption, OCR progress, print spooling notifications) directly into the bottom persistent status bar.

## [0.16.1] - 2026-09-21

### Fixed & Improved (GPUI-Kit Integration)
- **Visual & UX Redesign (GPUI-Kit)**:
  - Redesigned Welcome Hub with branded hero monogram, version badge, glowing drag-and-drop zone, 2×2 rich action cards (Open, Page Organizer, Digital Signatures, Security), and structured recent document cards.
  - Upgraded Ribbon toolbar with native desktop tab navigation, segmented action groups separated by hairline dividers, and muted section captions (`File`, `Page`, `Zoom`, `Layout`, `Markup`, `Shapes`, `Security`, etc.).
  - Added bottom persistent status bar with document geometry (`pt`), layout mode indicator, interactive page stepper, and quick zoom controls.
  - Enhanced document canvas with floating translucent page badges (`Page X of Y`), physical drop shadows, and refined tab strip styling.
- **Continuous Scroll & Viewport Navigation**:
  - Implemented stable virtual scroll container (`canvas-continuous-scroll`) with GPUI `ScrollHandle` and `.vertical_scrollbar(&self.scroll_handle)`.
  - Added full document page layout in continuous mode with dynamic top-item detection and pre-rendering as the user scrolls.
  - Linked thumbnail clicks to `scroll_handle.scroll_to_item(page_idx)` for smooth jumping across pages.
- **Ctrl + Mouse Wheel Zoom & Smooth Progressive Scaling**:
  - Eliminated the brief white flash during zooming by retaining existing GPU textures (`rendered_pages`) during zoom level changes instead of discarding them.
  - Implemented progressive scaling where GPUI's hardware-accelerated bilinear texture filter scales current page images in real time at 60+ FPS while higher-resolution rasterization proceeds asynchronously.
  - Added `rendered_zoom` tracking in `DocumentViewport` to seamlessly swap in crisp high-resolution renders without layout jumping or intermediate blank frames.
  - Handled rapid multi-step zooming gracefully with race condition prevention and in-flight render re-triggering.
- **Color Correction for Direct3D 11 Textures**:
  - Resolved inverted/reversed color balance (where red appeared blue and blue appeared red) by implementing `convert_rgba_to_bgra`.
  - Converted tiny-skia RGBA byte buffers to BGRA and un-premultiplied alpha as required by GPUI DirectX 11 textures.
- **Text Selection Highlighting & Word-Level Highlighting**:
  - Fixed text selection highlighting where selecting text did not visually highlight the words on the page.
  - Implemented precise word-level highlight quads (`window.paint_quad(fill(clipped_word, ...))`) painted directly over every intersecting `TextItem` in the page's text stream.
  - Added true coordinate translation between window pixels and PDF page points by tracking dynamic page card window origins (`viewport.page_origins`), ensuring selection accuracy regardless of window size, continuous scroll offset, or zoom level.
  - Corrected `TextItem` vertical coordinate calculation in `pdf_engine.rs` (`(page_height - span.y - span.size).max(0.0)`) to align with the top-left of font glyph bounding boxes rather than the baseline.
  - Added proactive text stream prefetching in `render_needed_pages` so text items and bounding boxes are immediately available when the user begins selecting.
  - Wired `RibbonTab::Annotate` -> `AnnotationTool::Highlight` and custom palette colors, allowing users to apply permanent yellow (or custom) highlights to selected text that persist across page interactions.
- **Mouse Drag Text Selection**:
  - Added mouse drag selection handlers (`on_mouse_down`, `on_mouse_move`, `on_mouse_up`) to canvas page cards.
  - Painted dynamic translucent blue selection highlight boxes over selected text via `canvas` quad clipping.
  - Integrated `PdfCommand::GetTextItems` to extract overlapping words within the selection bounding box and automatically copy to the system clipboard.
- **PDF Page Rasterization & Rendering Pipeline**:
  - Connected the multi-threaded `PdfEngine` worker pool (`spawn_engine_thread`) to the GPUI presentation layer.
  - Resolved the blank/white page display issue when opening PDF documents by piping `RenderResult` RGBA buffers into native GPUI `RenderImage` elements (`img(render_image).w_full().h_full()`).
  - Implemented asynchronous, non-blocking `request_render_page` pipeline using `cx.spawn` with automatic HiDPI scaling (1.5x multiplier).
  - Added live page thumbnail rasterization and rendering to the collapsible Sidebar page navigation list.
  - Wired zoom scaling, tab switching, and page navigation to synchronize viewport state and trigger automatic re-rendering.
- **Native File Dialog Integration**:
  - Wired native asynchronous file dialogs via `rfd::AsyncFileDialog` into GPUI's async task runner (`cx.spawn`).
  - Connected file browsing to the Welcome screen "Browse Files..." button, Welcome dropzone click, Ribbon Home tab "Open" action, and Tab Bar `+` button.
- **Full Drag-and-Drop Ingestion**:
  - Implemented `ExternalPaths` drop handling across both the dedicated Welcome dropzone and the global window surface.
  - Users can now drag and drop one or multiple PDF documents directly into PDFbull from Windows Explorer.
- **Windows Stack & Layout Stability**:
  - Expanded Windows linker stack reserve to 8 MB (`/STACK:8388608`) in `.cargo/config.toml` and `build.rs` to eliminate debug MSVC stack overflow.
  - Modularized Ribbon action strip rendering into `#[inline(never)]` helpers with heap-allocated element vectors.
  - Prevented circular layout negotiation loops by replacing `.size_full()` with `.flex_1().w_full()`.
- **GPUI Kit Design Guides Alignment**:
  - Audited against normative GPUI Kit Design Guides (`gpui-kit-design-guides`), ensuring semantic theme token usage (`cx.theme()`), desktop ergonomics, and clear visual state hierarchies.

## [0.16.0] - 2026-09-21

### Major UI Overhaul (GPUI-Kit)
- **Next-Gen GPU-Accelerated UI Architecture**:
  - Rewrote and expanded the application presentation layer to modern `gpui-kit` (`gpui-component` / GPUI), delivering buttery smooth GPU rendering and modern desktop aesthetics.
  - Built modular presentation sub-systems under `src/ui_gpui/`:
    - `app_view`: Top-level unified view (`PdfbullView`) composing header, ribbon strip, tabs, sidebar, canvas, developer log drawer, status bar, and modal overlays.
    - `ribbon`: Interactive 5-tab ribbon strip (Home, View, Annotate, Tools, Convert) with tool selector buttons and layout controls.
    - `tabs`: Multi-document tab bar with modified state indicators (`*`), close buttons, and tab switching.
    - `sidebar`: Collapsible sidebar with 6 navigation modes (Pages/Thumbnails, Bookmarks, Annotations, Fuzzy Search, Attachments, Layers).
    - `canvas`: Document viewport supporting multiple layout modes (Continuous, Single Page, Two-Page Spread), zoom scaling, and interactive page cards.
    - `dialogs`: Full-screen modal overlay system covering Watermark, Header/Footer, Security/Permissions, Password Prompt, Digital Signatures, Page Organizer, and Settings.
    - `welcome`: Modern drag-and-drop dropzone with quick-action cards (Merge PDFs, Page Organizer, Sign Document) and recent files.
    - `log_console`: Collapsible bottom developer drawer with severity filtering (All, Info, Warn, Error), copy all, and clear.
- **Zero Regressions & Backward Compatibility**:
  - Maintained 100% test coverage and zero regressions with all 153 unit, integration, OCR, and benchmark tests passing.
  - Retained CLI fallback flag (`--iced`) for instant switching to the classic Iced presentation engine.

## [0.15.3] - 2026-09-21

### Changed & Improved
- **Vector Icons System**:
  - Completely migrated UI from raw Unicode emojis to crisp, resolution-independent Lucide vector icons (`.font(LUCIDE)`).
  - Expanded `pub mod icons` with standard Lucide glyph codepoints (`HOME`, `EYE`, `PEN_TOOL`, `WRENCH`, `REFRESH_CW`, `MOON`, `SCROLL`, `FILE`, `BOOK_OPEN`, `BOOK`, `CIRCLE`, `MINUS`, `ARROW_RIGHT`, `STICKY_NOTE`, `LAYOUT_GRID`, `DROPLET`, `HASH`, `LOCK`, `SIGNATURE`, `SCISSORS`, `ZAP`, `FILE_PLUS`, `KEY`, `PALETTE`, `SCAN_TEXT`, `FILE_CODE`, `GLOBE`, `FILE_TEXT`, `IMAGE`, `PAPERCLIP`, `LAYERS`, `FOLDER`, `TRASH_2`, `LOADER`, `EXPAND_HORIZONTAL`, `MAXIMIZE`, `SPARKLES`, etc.).
  - Replaced all ribbon toolbar strip icons across View, Annotate, Tools, and Convert modes with vector glyphs.
  - Phased out and removed legacy `tool_button_emoji`.
  - Converted raw emoji color swatches into sleek circular dot containers with active glow halos.
  - Replaced emojis in Sidebar tabs (Thumbnails, Bookmarks, Annotations, Search, Attachments, Layers), Search HUD, canvas sticky notes, document loading spinners, and Welcome Quick Action cards.
  - Updated all modal dialog headers and action buttons (Watermark, Headers & Footers, Security & Permissions, Password Prompt, Digital Signature creator, Page Organizer, Signatures Inspector, Log Console) to native Lucide vector icons.
- **Dependency Updates**:
  - Updated project dependencies to latest compatible versions via `cargo update`.

## [0.15.1] - 2026-09-12

### Fixed & Optimized
- **Rust 2024 Concurrency & Win32 FFI Safety**:
  - Replaced deprecated `static mut` mutex handle with thread-safe `OnceLock<MutexHolder>` implementing RAII `Drop` cleanup.
  - Eliminated runtime heap allocation of UTF-16 strings in Win32 single-instance mutex and window detection using zero-cost `windows::core::w!` literals.
  - Added comprehensive `// SAFETY:` rationale comments across all Windows FFI call sites.
- **Robust Error Handling**:
  - Eliminated production `.unwrap()` call in document merge routing by adopting `Option::get_or_insert_with`.
- **Memory & Allocation Optimizations**:
  - Reused internal matching buffers in Command Palette fuzzy search (`filter_palette_items`), eliminating repetitive `format!` allocations.
  - Pre-allocated text assembly buffer in `OcrPageResult::full_text`, removing intermediate vector allocations.
  - Removed redundant string and path clones across tab session restoration and background command channels.
- **Idiomatic API Design**:
  - Implemented `From<SearchResultItem> for SearchResult` for standard trait conversions.

## [0.15.0] - 2026-09-08

### Added
- **Two-Page Spread / Book View & Standalone Cover Toggle**:
  - Dual-page side-by-side reading layout (`PageLayoutMode::TwoPageSpread`) designed for eBooks, magazines, and double-column papers.
  - Standalone cover toggle (`two_page_cover`) to present Page 1 centered as a standalone cover with subsequent pages displayed in two-page pairs.
  - View ribbon controls: `📖 Spread` mode toggle and `📕 Cover: On/Off` button.
  - Page navigation stepping by 2 pages in spread mode with proper cover alignment.
- **Continuous Scroll vs. Single-Page Presentation Mode**:
  - `PageLayoutMode::SinglePage` presentation mode focusing on a single active page centered in viewport without vertical overflow.
  - Ribbon selector buttons: `📜 Continuous` and `📄 Single` for instant switching.
  - Arrow keys (`Left`/`Right`) and `PgUp`/`PgDn` page snapping for distraction-free reading.
- **Tab Ergonomics**:
  - **Middle-Click to Close Tab**: Middle-clicking any tab directly closes it.
  - **Right-Click Tab Context Menu**: Context menu offering `Close Tab`, `Close Others`, `Close Tabs to the Right`, `Copy File Path`, and `Open Containing Folder`.
  - **Drag-and-Drop PDF File Opening**: Drag any PDF file directly into the PDFbull window canvas to open it in a new tab, with visual drop overlay indicator.
- **Interactive Drag to Select Text**:
  - Native `Text` (I-beam) cursor hovering over pages in pointer mode.
  - Dynamic click-and-drag marquee bounding box with live word highlighting during dragging.
  - Rotation-independent (0°, 90°, 180°, 270°) and zoom-independent coordinate space mapping.
  - Multi-line smart formatting: lines grouped by baseline, ordered horizontally, separated by newlines, with merged highlight bounding boxes per line segment.
  - Automatic clipboard copy on mouse release, `Ctrl+C` manual copy, and `H` / `Ctrl+H` one-key permanent PDF highlight annotation creation.
  - Click outside (< 3px) to dismiss selection.

## [0.14.0] - 2026-08-25

### Added
- **In-App Developer Log Console (`src/ui_log_console.rs` & `src/logging.rs`)**:
  - Thread-safe 1,000-entry memory-backed circular ring buffer (`RingBufferLayer`) capturing live structured events across engine workers and UI threads.
  - Interactive bottom drawer panel with real-time level filtering (`All`, `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`), 1-click **Copy All** to clipboard, buffer clearing, and automatic scroll-to-bottom toggle.
  - Color-coded monospace log viewer with distinct severity styling (Red errors, amber warnings, cyan info, and slate debug/trace).
  - New keyboard shortcut **`Ctrl+L`**, Command Palette entry (`Toggle Log Console`), and **`🖥️ Logs`** toolbar button.
  - Startup persistence: added `show_logs_on_start` setting in `AppSettings` configurable via the Settings modal.
- **Comprehensive PDF Conformance Validator Suite**: Added end-to-end conformance validation powered by `zpdf = "=0.13.0"` supporting 9 international standard profiles:
  - **PDF/A**: `PDF/A-1b` (ISO 19005-1), `PDF/A-2b` (ISO 19005-2), and `PDF/A-3b` (ISO 19005-3 archival with embedded files).
  - **PDF/X**: `PDF/X-1a` (ISO 15930-1), `PDF/X-3` (ISO 15930-3), `PDF/X-4` (ISO 15930-7), and `PDF/X-6` (ISO 15930-9 prepress with live transparency and layers).
  - **PDF/UA**: `PDF/UA-1` (ISO 14289-1) and `PDF/UA-2` (ISO 14289-2 universal accessibility).
- **Interactive Conformance Validation Modal**: Dedicated UI modal (`src/ui_conformance.rs`) featuring standard profile cards, 1-click async validation runner, conformance status banner (✓/✗), claimed metadata tag extraction, and scrollable rule violation diagnostics.
- **Entry Points & Command Palette**: Added `🛡️ Validate` button to the main tools toolbar and integrated `Validate PDF Conformance` into the fuzzy Command Palette (`Ctrl+K`).
- **Generalized Conformance Data Model**: Replaced legacy dead-code PDF/A report with `ConformanceReport` and `ConformanceFamily`, normalizing claimed standard metadata across PDF/A (`pdfaid`), PDF/X (`GTS_PDFXVersion`), and PDF/UA.

### Fixed
- **Resolved Stray Release Terminal Window**: Moved `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` to `src/main.rs:1` (binary root), eliminating unwanted Windows console window popups on release builds while maintaining full debugging stdout output during development.
- **Panic Hook Log Stream Integration**: Wired panic handlers directly into `tracing::error!(target: "panic", ...)` so unexpected panics immediately appear in the in-app log console.

## [0.13.7] - 2026-08-24

### Removed & Replaced
- **Replaced `dark-light` with Native Win32 Registry Inspection**: Removed `dark-light` dependency (and its transitive `async-std` / `zbus` trees), replacing it with a zero-dependency Win32 `RegGetValueW` registry check on `AppsUseLightTheme` in `src/platform/windows.rs`.
- **Resolved `cargo-deny` Advisories & Licenses**: Fixed `deny.toml` for `cargo-deny` 0.16+ / 0.20+, allowed all permissive transitive licenses (`0BSD`, `BSL-1.0`, `IJG`, `NCSA`, `CDLA-Permissive-2.0`), and purged unneeded advisory exclusions.

## [0.13.6] - 2026-08-24

### Fixed
- **Tracing Worker Guard Lifetime**: Maintained non-blocking `WorkerGuard` for the full application lifecycle via `Box::leak`, guaranteeing file log persistence and eliminating panic race conditions in production.
- **Genuine OCR Text Recognition Engine**: Replaced synthetic layout approximations with real `ocrs::OcrEngine` and `rten` detection/recognition neural network inference and PDF user space coordinate mapping.
- **`OcConfig` Memory Safety & Pinned Layout**: Pinned `zpdf = "=0.12.1"` and `zpdf-writer = "=0.12.1"` and added compile-time static assertions validating `zpdf::OcConfig` size and alignment against internal representations.
- **Rotation-Aware Render Cache**: Included page rotation angle in `RenderKey` and centralized multi-worker cache invalidation and document removal in `RenderCache`.
- **Memory & Resource Protection Limits**: Enforced 512 MB file size limit checks prior to memory reads in both `open_document` and `convert_pdf_doc`.
- **Session Restore Tab Synchronization**: Attached `pending_session` directly to newly opened tabs during restore to prevent race conditions with background document loading.
- **Password Zeroization**: Guaranteed immediate zeroization of sensitive password buffers upon failed authentication attempts.
- **Single-Instance Multi-File Launching**: Ensured all command-line arguments are transmitted and handled over local IPC pipes without dropping secondary file arguments.
- **Cross-Filesystem Atomic Writes**: Added reliable fallback in `atomic_write` for cross-device links, NTFS junctions, and mount boundaries.
- **Panic Hook Chaining**: Properly chained existing custom panic handlers after `human_panic` initialization.

### Changed & Improved
- **PNG Compression Optimization**: Optimized PNG export compression via `oxipng` preset 2.
- **CI Benchmarking Coverage**: Added automated benchmark compilation checks (`cargo check --benches`) in CI workflows.
- **Dependency Upgrades**: Upgraded dependencies to latest compatible versions including `blocking 1.7`, `cc 1.4`, `h2 0.4.18`, `uuid 1.25`, and `libdeflater 1.26`.

## [0.13.5] - 2026-08-20

### Fixed
- **OCG Layer Toggle Live Rendering**: Implemented safe runtime visibility override application to `zpdf::OcConfig` via `OcConfigInternal::apply_overrides`, enabling interactive Layer toggle state changes in the UI to immediately take visual effect during rendering.
- **Robust Hex & CSS Color Parsing**: Added `try_hex_to_rgb` and `hex_to_rgb_or` supporting 3-digit (`#FFF`), 6-digit (`#RRGGBB`), 8-digit (`#RRGGBBAA`), bare hex, and named CSS colors, returning `None` / configurable fallback on parse failure instead of silently defaulting to black `(0,0,0)`.
- **Pixel-Perfect Sidebar Thumbnails**: Fixed thumbnail zoom render width from 120px to 160px (`thumb_zoom`), matching the fixed 160px sidebar container and eliminating blurry upscaling artifacts.
- **Strict Clippy Compliance & Code Quality**: Removed all 44 blanket `#![allow(...)]` attributes from `src/lib.rs`, modernized `[lints.clippy]` in `Cargo.toml`, and resolved all compiler and clippy warnings across library, test, and binary targets under `-D warnings`.
- **Security Hardening with `zeroize`**: Replaced hand-rolled `write_bytes` with the `zeroize` crate in `src/models.rs`, guaranteeing compiler dead-store elimination immunity for sensitive password data in memory.
- **Asynchronous Tracing with `tracing-appender`**: Replaced bespoke blocking `DualWriter` in `src/main.rs` with `tracing_appender::rolling::never` and non-blocking multi-writer log streaming.
- **Typed Windows FFI Bindings**: Replaced raw `unsafe extern "system"` FFI block in `src/platform/windows.rs` with official typed bindings from `windows::Win32::UI::WindowsAndMessaging` and `Win32_System_Threading`.
- **Dependency Footprint Pruning**: Removed unused `image 0.25`, direct redundant `notify 8`, and unused dev-dependency `sha2 0.11`, significantly reducing build dependencies.

## [0.13.0] - 2026-08-14

### Added
- **Interactive Command Palette (`Ctrl + K` / `Ctrl + Shift + P`)**: Floating quick-action modal with fuzzy matching powered by `nucleo-matcher` for instant navigation, tools, document outlines, theme switching, and format exports.
- **Dependency & License Auditing (`cargo-deny`)**: Added `deny.toml` configuration and integrated automated security vulnerability / license compliance scanning in GitHub Actions CI.
- **Fuzzy Search Integration (`nucleo-matcher 0.3`)**: Added allocation-free fuzzy string filtering for bookmarks, search navigation, and action palettes.

### Changed & Improved
- **Modernized Atomic File Writes**: Replaced unmaintained `atomicwrites` crate with `tempfile::NamedTempFile` for robust, cross-platform atomic writes of application settings and session data.
- **Tokio Dependency Footprint Optimization**: Pruned `tokio` features from `"full"` to exact required modules (`rt-multi-thread`, `sync`, `time`, `process`, `fs`, `io-util`, `macros`), accelerating incremental builds.
- **Persistent Image Caching in `PdfEngine`**: Made per-document `ImageCache` persistent across page renders to eliminate redundant decompression of embedded PDF image streams during scrolling.
- **Font Asset Integrity**: Replaced corrupted HTML placeholder files with valid TrueType font binaries in `src/assets/fonts/` and purged obsolete font files.

## [0.12.2] - 2026-08-08

### Added
- **`zpdf` & `zpdf-writer` v0.12.0 Upgrade**: Upgraded core rendering and writer dependencies to `zpdf 0.12.0` workspace chain.
- **PDF/UA Accessibility Tagging (`tag_pdf`)**: Exposed `DocumentStore::tag_pdf` API to inject structural `/StructTreeRoot` and `/ParentTree` tags into untagged PDFs.
- **Appearance Stream Baking (`/AP` `/N`)**: Baked appearance streams for annotated elements to ensure visual fidelity across external PDF viewers (Adobe Acrobat, Preview, Chrome).
- **Dehyphenation & Logical RTL Ordering**: Integrated automatic dehyphenation (`coopera-\ntion` ➔ `cooperation`) and visual-to-logical Hebrew/Arabic RTL run reversal in text extraction.
- **Memory-Mapped ONNX Model Loading (`rten` `mmap`)**: Enabled zero-copy `mmap` model loading for OCR neural network inference.

## [0.11.5] - 2026-08-08

### Fixed
- **Real PDF Stream Compression**: Enabled `compress_uncompressed = true` and 1600px image downsampling in `DocumentStore::optimize_pdf()`, enabling 30%–70% PDF file size reductions across text, vector, and image PDF files.
- **UI Thread Disk Stalls**: Offloaded `save_settings`, `save_recent_files`, and `save_session` disk write operations to background threads, eliminating UI frame drops during state updates.

### Added
- **Accessibility & Contrast Compliance**: Refactored theme color tokens (`COLOR_TEXT_DIM`, `COLOR_TEXT_SECONDARY`) for WCAG 2.1 AA contrast compliance and added visual focus ring indicators for button states.
- **Welcome Screen Hotkey Hints**: Integrated dynamic package versioning (`env!("CARGO_PKG_VERSION")`) and keyboard shortcut hints (`Ctrl + O`) across Welcome screen cards.

## [0.11.0] - 2026-07-29

### Added
- **Dynamic Header & Page Numbering**: Injected top header text and formatted page numbers (`"Page {page} of {pages}"`) directly into PDF graphics streams via `zpdf-writer`.
- **Granular Security & Permission Control**: Added security rules configuration dialog for controlling printing, text copying, and document editing permissions.
- **Open Containing Folder**: Added 📁 **Open Folder** button in tools ribbon using `open::that(...)` to reveal the active PDF's location in Windows File Explorer.
- **Document-Wide Multi-Page OCR**: Integrated `ocr_document_parallel` method to run full-document text recognition across all pages.
- **NSIS Setup Installer GUI Branding**: Added custom dark-themed installer sidebar graphics (`welcome.bmp`), top header banner (`header.bmp`), custom icon (`PDFbull.ico`), and automatic `.rten` model packaging in `PDFbull-Setup.exe`.
- **License Documentation Update**: Refreshed `THIRD-PARTY-LICENSES.md` to accurately document `iced`, `zpdf`, `ocrs`, and `rten` licenses.

## [0.10.5] - 2026-07-28

### Fixed
- **Pre-OCRed & Rotated PDF Search Hit Alignment**: Fixed search result hit bounding box math by passing `.with_page_rotation(page.rotate)` into `ContentInterpreter` and calculating normalized top-down Y offsets (`eff_box.y1 - span.y - span.size`).

### Added
- **GUI Convert Ribbon Tab**: Added `🔄 Convert` ribbon tab in top toolbar with 1-click action buttons for exporting documents to Markdown (`.md`), HTML5 (`.html`), and Plain Text (`.txt`).
- **OCR Unit Test Suite**: Expanded `tests/ocr_test.rs` to 18 automated tests covering OCR data models, Devanagari text processing, Serde JSON serialization, and export command payloads.

## [0.10.4] - 2026-07-28

### Added
- **Multi-Script Devanagari & Latin OCR Engine (`OcrScript`)**: Integrated `OcrScript` enum enabling full runtime script selection for Devanagari (Hindi, Marathi, Sanskrit, Nepali) via `devanagari_PP-OCRv4_rec.rten` and Latin/English script via `text-recognition.rten`.
- **Command & Messaging Pipeline Updates**: Updated `PdfCommand::OcrPage(doc_id, page_num, script, tx)` and `Message::SelectOcrScript(script)` handlers with unit tests (`tests/ocr_test.rs`).

## [0.10.3] - 2026-07-28

### Added
- **Pure-Rust OCR Capability (`ocrs` + `rten`)**: Built-in text recognition and bounding box extraction for scanned / image-only PDF pages (`PdfCommand::OcrPage`) with zero C++ DLL dependencies. Out-of-the-box support for **Devanagari** (Hindi, Marathi, Sanskrit) and **Latin** (English, European) scripts.
- **OCR Toolbar & Service Integration**: Added 🔍 **OCR Page** tool action button in the Tools ribbon with background engine processing and automated test suite (`tests/ocr_test.rs`).

## [0.10.2] - 2026-07-28

### Added
- **New Blank PDF Document Creation (`DocumentBuilder`)**: Programmatically generate new blank A4 PDF documents with custom initial layout (`PdfCommand::CreateBlankDocument`).
- **Digital Certificate Signing (`SigningKey`)**: Apply cryptographic PKCS#8 / PKCS#12 digital signatures to PDFs via `zpdf_writer` with interactive cert picker modal (`PdfCommand::SignDocumentWithCert`).
- **Rubber Stamp Annotations (`StampItem`)**: Overlay styled rubber stamps (`APPROVED`, `CONFIDENTIAL`, `DRAFT`, `REJECTED`, `FINAL`) directly on PDF pages (`PdfCommand::ApplyStamp`).
- **Geospatial GIS Metadata Panel (`Measure` / `/GCS`)**: Inspect `/Measure` & `/GCS` dictionaries on GeoPDF files, displaying coordinate systems, EPSG codes, WKT projections, and distance units.
- **CMYK & Prepress Color Inspector (`output_intent_cmyk_profile`)**: Inspect embedded ICC color profiles and document `/OutputIntents`, plus live CMYK ↔ RGB converter panel (`cmyk_to_rgb_naive` & `rgb_to_cmyk_naive`) with maximum GCR.

## [0.10.1] - 2026-07-28

### Performance
- **Pre-decoded 0ms Window Icon**: Replaced runtime ICO file parsing with pre-decoded 32x32 raw RGBA pixel buffer (`src/assets/icon_32x32.rgba`), saving ~15–25ms on main thread cold launch.

## [0.10.0] - 2026-07-27

### Added
- **PDF Encryption on Save**: Encrypt PDFs with AES-256 (V5/R6) or RC4-128 (V2/R3) security handlers and custom user/owner passwords (`PdfCommand::EncryptPdf`).
- **PDF/A Conformance Validation**: Built-in validation suite for PDF/A-1b and PDF/A-2b compliance standards (`PdfCommand::ValidatePdfA`).
- **Signature Trust Chain Verification**: X.509 certificate chain validation for PDF signatures against custom PEM/DER root trust anchors (`PdfCommand::VerifySignatureTrust`).
- **Linearization / Fast Web View**: Reorganizes PDF object structures and xref tables for ISO 32000-1 Annex F web streaming (`PdfCommand::LinearizePdf`).
- **Document Export & Conversion**: Multi-format export engine (`convert_pdf`) supporting Markdown, semantic HTML5, and raw TXT conversions in TextOnly or Rich modes (`PdfCommand::ConvertPdf`).
- **Image Downsampling in Optimization**: Integrated `max_image_dimension(2400)` option in structural PDF optimization pass (`optimize_pdf`) for shrinking large PDF files.

### Fixed
- **Release CI Workflow**: Corrected workflow trigger from `pull_request` to tag push (`push.tags`), updated GitHub Action versions (`checkout@v4`, `upload-artifact@v4`, `download-artifact@v4`).
- **Engine Compile Errors**: Resolved premature brace closure in `pdf_engine.rs`, aligned `zpdf` 0.11 API calls for `IncrementalWriter`, `rewrite_pdf`, `RedactOptions`, and `Profile`.
- **Clippy Cleanliness**: Fixed all `doc_markdown`, `uninlined_format_args`, `format_push_string`, and `items_after_statements` lints across library and tests.

---

## [0.9.1] - 2026-07-19

### Added
- **PDFgear-Inspired Ribbon GUI Overhaul**: Modernized header ribbon toolbar with tabs for File, Home, Annotate, Convert, Tools, and View.
- **Performance & Startup Optimization**: Sub-1-second cold launch times and optimized page caching.

---

## [0.9.0] - 2026-07-10

### Added
- **Table Extraction & Bounding Box UI**: Automatic grid detection, cell outlines, and 1-click CSV/TSV copy actions.
- **Digital Signatures Verification**: Signature dictionary parsing and byte-range digest validation.
- **Attachments Panel**: Dedicated sidebar tab for listing and saving embedded files.
- **Layers Manager (OCProperties)**: Optional Content Group visibility toggling in sidebar.
- **Password-Protected PDF Support**: Prompt and decrypt password-protected PDFs on open.
- **Tagged PDF Support**: Structured reading order text extraction.

