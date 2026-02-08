/*
 * PDF Font FFI
 *
 * Provides PDF-specific font handling including font descriptors,
 * CID/GID/Unicode mapping, metrics, and font embedding.
 */

#ifndef MICROPDF_PDF_FONT_H
#define MICROPDF_PDF_FONT_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_obj;
typedef uint64_t fz_font;
typedef uint64_t pdf_font_desc;
typedef uint64_t pdf_cmap;
typedef uint64_t fz_output;

/* ============================================================================
 * Font Descriptor Flags
 * ============================================================================ */

#define PDF_FD_FIXED_PITCH  (1 << 0)
#define PDF_FD_SERIF        (1 << 1)
#define PDF_FD_SYMBOLIC     (1 << 2)
#define PDF_FD_SCRIPT       (1 << 3)
#define PDF_FD_NONSYMBOLIC  (1 << 5)
#define PDF_FD_ITALIC       (1 << 6)
#define PDF_FD_ALL_CAP      (1 << 16)
#define PDF_FD_SMALL_CAP    (1 << 17)
#define PDF_FD_FORCE_BOLD   (1 << 18)

/* ============================================================================
 * Font Encoding Constants
 * ============================================================================ */

#define PDF_ENCODING_STANDARD       0
#define PDF_ENCODING_MAC_ROMAN      1
#define PDF_ENCODING_WIN_ANSI       2
#define PDF_ENCODING_MAC_EXPERT     3
#define PDF_ENCODING_SYMBOL         4
#define PDF_ENCODING_ZAPF_DINGBATS  5

/* ============================================================================
 * CJK Script Constants
 * ============================================================================ */

#define PDF_CJK_CNS1    0   /* Traditional Chinese */
#define PDF_CJK_GB1     1   /* Simplified Chinese */
#define PDF_CJK_JAPAN1  2   /* Japanese */
#define PDF_CJK_KOREA1  3   /* Korean */

/* ============================================================================
 * Metrics Structures
 * ============================================================================ */

/** Horizontal metrics entry */
typedef struct {
    uint16_t lo;
    uint16_t hi;
    int32_t w;
} pdf_hmtx;

/** Vertical metrics entry */
typedef struct {
    uint16_t lo;
    uint16_t hi;
    int16_t x;
    int16_t y;
    int16_t w;
} pdf_vmtx;

/* ============================================================================
 * Font Descriptor Lifecycle
 * ============================================================================ */

/**
 * Create a new font descriptor.
 * @return Font descriptor handle
 */
pdf_font_desc *pdf_new_font_desc(fz_context *ctx);

/**
 * Keep (increment reference to) a font descriptor.
 * @return Same font handle
 */
pdf_font_desc *pdf_keep_font(fz_context *ctx, pdf_font_desc *font);

/**
 * Drop a font descriptor.
 */
void pdf_drop_font(fz_context *ctx, pdf_font_desc *font);

/* ============================================================================
 * Font Properties
 * ============================================================================ */

/**
 * Get font name.
 * @return Name string (caller must free with pdf_font_free_string)
 */
const char *pdf_font_name(fz_context *ctx, pdf_font_desc *font);

/**
 * Set font name.
 */
void pdf_set_font_name(fz_context *ctx, pdf_font_desc *font, const char *name);

/**
 * Get font flags.
 * @return Combination of PDF_FD_* flags
 */
int pdf_font_flags(fz_context *ctx, pdf_font_desc *font);

/**
 * Set font flags.
 */
void pdf_set_font_flags(fz_context *ctx, pdf_font_desc *font, int flags);

/**
 * Get italic angle.
 */
float pdf_font_italic_angle(fz_context *ctx, pdf_font_desc *font);

/**
 * Get ascent.
 */
float pdf_font_ascent(fz_context *ctx, pdf_font_desc *font);

/**
 * Get descent.
 */
float pdf_font_descent(fz_context *ctx, pdf_font_desc *font);

/**
 * Get cap height.
 */
float pdf_font_cap_height(fz_context *ctx, pdf_font_desc *font);

/**
 * Get x-height.
 */
float pdf_font_x_height(fz_context *ctx, pdf_font_desc *font);

/**
 * Get missing glyph width.
 */
float pdf_font_missing_width(fz_context *ctx, pdf_font_desc *font);

/**
 * Check if font is embedded.
 * @return 1 if embedded, 0 otherwise
 */
int pdf_font_is_embedded(fz_context *ctx, pdf_font_desc *font);

/* ============================================================================
 * Writing Mode
 * ============================================================================ */

/**
 * Get font writing mode.
 * @return 0 for horizontal, 1 for vertical
 */
int pdf_font_wmode(fz_context *ctx, pdf_font_desc *font);

/**
 * Set font writing mode.
 */
void pdf_set_font_wmode(fz_context *ctx, pdf_font_desc *font, int wmode);

/* ============================================================================
 * Metrics Functions
 * ============================================================================ */

/**
 * Set default horizontal metrics.
 * @param w Default width
 */
void pdf_set_default_hmtx(fz_context *ctx, pdf_font_desc *font, int w);

/**
 * Set default vertical metrics.
 * @param y Default Y displacement
 * @param w Default width
 */
void pdf_set_default_vmtx(fz_context *ctx, pdf_font_desc *font, int y, int w);

/**
 * Add horizontal metrics entry.
 * @param lo Low CID
 * @param hi High CID
 * @param w Width
 */
void pdf_add_hmtx(fz_context *ctx, pdf_font_desc *font, int lo, int hi, int w);

/**
 * Add vertical metrics entry.
 * @param lo Low CID
 * @param hi High CID
 * @param x X displacement
 * @param y Y displacement
 * @param w Width
 */
void pdf_add_vmtx(fz_context *ctx, pdf_font_desc *font, int lo, int hi, int x, int y, int w);

/**
 * Finalize horizontal metrics (sorts table).
 */
void pdf_end_hmtx(fz_context *ctx, pdf_font_desc *font);

/**
 * Finalize vertical metrics (sorts table).
 */
void pdf_end_vmtx(fz_context *ctx, pdf_font_desc *font);

/**
 * Lookup horizontal metrics for a CID.
 * @return Metrics entry
 */
pdf_hmtx pdf_lookup_hmtx(fz_context *ctx, pdf_font_desc *font, int cid);

/**
 * Lookup vertical metrics for a CID.
 * @return Metrics entry
 */
pdf_vmtx pdf_lookup_vmtx(fz_context *ctx, pdf_font_desc *font, int cid);

/* ============================================================================
 * CID Mapping Functions
 * ============================================================================ */

/**
 * Map CID to GID (glyph ID).
 * @return GID for the given CID
 */
int pdf_font_cid_to_gid(fz_context *ctx, pdf_font_desc *font, int cid);

/**
 * Map CID to Unicode.
 * @return Unicode codepoint for the given CID
 */
int pdf_font_cid_to_unicode(fz_context *ctx, pdf_font_desc *font, int cid);

/**
 * Set CID to GID mapping table.
 * @param table Array of GID values indexed by CID
 * @param len Length of table
 */
void pdf_set_cid_to_gid(fz_context *ctx, pdf_font_desc *font, const uint16_t *table, size_t len);

/**
 * Set CID to UCS mapping table.
 * @param table Array of Unicode values indexed by CID
 * @param len Length of table
 */
void pdf_set_cid_to_ucs(fz_context *ctx, pdf_font_desc *font, const uint16_t *table, size_t len);

/* ============================================================================
 * Font Loading Functions
 * ============================================================================ */

/**
 * Load font from PDF document.
 * @param doc Document handle
 * @param rdb Resource database handle
 * @param obj Font dictionary object
 * @return Font descriptor handle
 */
pdf_font_desc *pdf_load_font(fz_context *ctx, pdf_document *doc, void *rdb, pdf_obj *obj);

/**
 * Load Type 3 font from PDF document.
 * @return Font descriptor handle
 */
pdf_font_desc *pdf_load_type3_font(fz_context *ctx, pdf_document *doc, void *rdb, pdf_obj *obj);

/**
 * Load Type 3 glyphs.
 */
void pdf_load_type3_glyphs(fz_context *ctx, pdf_document *doc, pdf_font_desc *font);

/**
 * Load fallback "hail mary" font.
 * @return Font descriptor handle
 */
pdf_font_desc *pdf_load_hail_mary_font(fz_context *ctx, pdf_document *doc);

/* ============================================================================
 * Encoding Functions
 * ============================================================================ */

/**
 * Load encoding strings.
 * @param estrings Array to fill with encoding strings
 * @param encoding Encoding name
 */
void pdf_load_encoding(const char **estrings, const char *encoding);

/* ============================================================================
 * Substitute Font Functions
 * ============================================================================ */

/**
 * Lookup substitute font.
 * @param mono Monospace flag
 * @param serif Serif flag
 * @param bold Bold flag
 * @param italic Italic flag
 * @param len Pointer to receive font data length
 * @return Pointer to font data (or NULL)
 */
const unsigned char *pdf_lookup_substitute_font(fz_context *ctx, int mono, int serif, int bold, int italic, int *len);

/**
 * Clean font name (remove subset prefixes).
 * @return Cleaned name (caller must free with pdf_font_free_string)
 */
const char *pdf_clean_font_name(const char *fontname);

/* ============================================================================
 * Font Addition Functions
 * ============================================================================ */

/**
 * Add simple font to document.
 * @param encoding PDF_ENCODING_* constant
 * @return Font dictionary object handle
 */
pdf_obj *pdf_add_simple_font(fz_context *ctx, pdf_document *doc, fz_font *font, int encoding);

/**
 * Add CID font to document.
 * @return Font dictionary object handle
 */
pdf_obj *pdf_add_cid_font(fz_context *ctx, pdf_document *doc, fz_font *font);

/**
 * Add CJK font to document.
 * @param script PDF_CJK_* constant
 * @param wmode Writing mode
 * @param serif Serif flag
 * @return Font dictionary object handle
 */
pdf_obj *pdf_add_cjk_font(fz_context *ctx, pdf_document *doc, fz_font *font, int script, int wmode, int serif);

/**
 * Add substitute font to document.
 * @return Font dictionary object handle
 */
pdf_obj *pdf_add_substitute_font(fz_context *ctx, pdf_document *doc, fz_font *font);

/**
 * Check if font writing is supported.
 * @return 1 if supported, 0 otherwise
 */
int pdf_font_writing_supported(fz_context *ctx, fz_font *font);

/**
 * Subset fonts in document.
 * @param pages_len Number of pages (0 for all)
 * @param pages Array of page indices
 */
void pdf_subset_fonts(fz_context *ctx, pdf_document *doc, int pages_len, const int *pages);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/**
 * Print font information to output stream.
 */
void pdf_print_font(fz_context *ctx, fz_output *out, pdf_font_desc *font);

/**
 * Free a string allocated by font functions.
 */
void pdf_font_free_string(fz_context *ctx, char *s);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_FONT_H */


