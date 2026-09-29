# Changelog

All notable changes to the PDFbull project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

