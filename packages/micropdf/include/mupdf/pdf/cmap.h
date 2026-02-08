/*
 * PDF CMap FFI
 *
 * Provides Character Map (CMap) support for PDF text encoding,
 * including CID/Unicode mapping and vertical writing mode.
 */

#ifndef MICROPDF_PDF_CMAP_H
#define MICROPDF_PDF_CMAP_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_obj;
typedef uint64_t fz_stream;
typedef uint64_t pdf_cmap;

/* ============================================================================
 * Constants
 * ============================================================================ */

/** Maximum 1-to-many mapping length (256 characters for ToUnicode CMaps) */
#define PDF_MRANGE_CAP 256

/** Maximum codespace entries */
#define PDF_CODESPACE_MAX 40

/* ============================================================================
 * Writing Mode
 * ============================================================================ */

/** Writing mode enumeration */
typedef enum {
    PDF_WMODE_HORIZONTAL = 0,
    PDF_WMODE_VERTICAL = 1
} pdf_wmode;

/* ============================================================================
 * Range Structures
 * ============================================================================ */

/** Simple range mapping (16-bit) */
typedef struct {
    uint16_t low;
    uint16_t high;
    uint16_t out;
} pdf_range;

/** Extended range mapping (32-bit) */
typedef struct {
    uint32_t low;
    uint32_t high;
    uint32_t out;
} pdf_xrange;

/** One-to-many range mapping */
typedef struct {
    uint32_t low;
    uint32_t out;
} pdf_mrange;

/* ============================================================================
 * CMap Lifecycle Functions
 * ============================================================================ */

/**
 * Create a new empty CMap.
 * @return CMap handle
 */
pdf_cmap *pdf_new_cmap(fz_context *ctx);

/**
 * Keep (increment reference to) a CMap.
 * @return Same CMap handle
 */
pdf_cmap *pdf_keep_cmap(fz_context *ctx, pdf_cmap *cmap);

/**
 * Drop a CMap.
 */
void pdf_drop_cmap(fz_context *ctx, pdf_cmap *cmap);

/**
 * Get CMap size in memory.
 * @return Size in bytes
 */
size_t pdf_cmap_size(fz_context *ctx, pdf_cmap *cmap);

/* ============================================================================
 * CMap Properties
 * ============================================================================ */

/**
 * Get CMap name.
 * @return Name string (caller must free with pdf_cmap_free_string)
 */
const char *pdf_cmap_name(fz_context *ctx, pdf_cmap *cmap);

/**
 * Set CMap name.
 * @param name New name
 */
void pdf_set_cmap_name(fz_context *ctx, pdf_cmap *cmap, const char *name);

/**
 * Get CMap writing mode.
 * @return 0 for horizontal, 1 for vertical
 */
int pdf_cmap_wmode(fz_context *ctx, pdf_cmap *cmap);

/**
 * Set CMap writing mode.
 * @param wmode 0 for horizontal, 1 for vertical
 */
void pdf_set_cmap_wmode(fz_context *ctx, pdf_cmap *cmap, int wmode);

/**
 * Set UseCMap (parent CMap for cascading).
 * @param usecmap Parent CMap handle
 */
void pdf_set_usecmap(fz_context *ctx, pdf_cmap *cmap, pdf_cmap *usecmap);

/* ============================================================================
 * Codespace Functions
 * ============================================================================ */

/**
 * Add a codespace range.
 * Codespace ranges define valid multi-byte sequences.
 * @param low Low value of range
 * @param high High value of range
 * @param n Number of bytes
 */
void pdf_add_codespace(fz_context *ctx, pdf_cmap *cmap, unsigned int low, unsigned int high, size_t n);

/**
 * Get number of codespace entries.
 * @return Number of codespace entries
 */
int pdf_cmap_codespace_len(fz_context *ctx, pdf_cmap *cmap);

/* ============================================================================
 * Mapping Functions
 * ============================================================================ */

/**
 * Map a range of codepoints to another range.
 * For example, mapping 0x20-0x7E to CIDs 1-95.
 * @param srclo Source range low
 * @param srchi Source range high
 * @param dstlo Destination range start
 */
void pdf_map_range_to_range(fz_context *ctx, pdf_cmap *cmap, unsigned int srclo, unsigned int srchi, int dstlo);

/**
 * Map one codepoint to many.
 * Used for ligatures and ToUnicode mappings.
 * @param one Source codepoint
 * @param many Array of destination codepoints
 * @param len Length of many array (max PDF_MRANGE_CAP)
 */
void pdf_map_one_to_many(fz_context *ctx, pdf_cmap *cmap, unsigned int one, int *many, size_t len);

/**
 * Sort CMap for efficient lookup.
 */
void pdf_sort_cmap(fz_context *ctx, pdf_cmap *cmap);

/* ============================================================================
 * Lookup Functions
 * ============================================================================ */

/**
 * Lookup a codepoint.
 * @param cpt Input codepoint
 * @return Mapped codepoint (or input if no mapping)
 */
int pdf_lookup_cmap(pdf_cmap *cmap, unsigned int cpt);

/**
 * Lookup a codepoint with full output.
 * Returns one-to-many mappings if present.
 * @param cpt Input codepoint
 * @param out Pointer to receive first output codepoint
 * @return Number of output codepoints
 */
int pdf_lookup_cmap_full(pdf_cmap *cmap, unsigned int cpt, int *out);

/**
 * Decode a multi-byte encoded string.
 * Uses codespace ranges to extract codepoints.
 * @param s Start of string
 * @param e End of string
 * @param cpt Pointer to receive decoded codepoint
 * @return Number of bytes consumed
 */
int pdf_decode_cmap(pdf_cmap *cmap, unsigned char *s, unsigned char *e, unsigned int *cpt);

/* ============================================================================
 * Identity CMap Functions
 * ============================================================================ */

/**
 * Create an Identity CMap.
 * @param wmode Writing mode (0=horizontal, 1=vertical)
 * @param bytes Bytes per codepoint (1 or 2)
 * @return CMap handle
 */
pdf_cmap *pdf_new_identity_cmap(fz_context *ctx, int wmode, int bytes);

/* ============================================================================
 * Load CMap Functions
 * ============================================================================ */

/**
 * Load a built-in CMap by name.
 * Supports Identity-H, Identity-V, and standard Adobe CMaps.
 * @param name CMap name
 * @return CMap handle, or 0 if not found
 */
pdf_cmap *pdf_load_builtin_cmap(fz_context *ctx, const char *name);

/**
 * Load a system CMap by name.
 * @param name CMap name
 * @return CMap handle, or 0 if not found
 */
pdf_cmap *pdf_load_system_cmap(fz_context *ctx, const char *name);

/**
 * Load CMap from a stream.
 * @param file Stream handle
 * @return CMap handle
 */
pdf_cmap *pdf_load_cmap(fz_context *ctx, fz_stream *file);

/**
 * Load embedded CMap from PDF document.
 * @param doc Document handle
 * @param ref PDF object reference
 * @return CMap handle
 */
pdf_cmap *pdf_load_embedded_cmap(fz_context *ctx, pdf_document *doc, pdf_obj *ref);

/* ============================================================================
 * Information Functions
 * ============================================================================ */

/**
 * Get number of simple ranges.
 */
int pdf_cmap_range_count(fz_context *ctx, pdf_cmap *cmap);

/**
 * Get number of extended ranges.
 */
int pdf_cmap_xrange_count(fz_context *ctx, pdf_cmap *cmap);

/**
 * Get number of one-to-many ranges.
 */
int pdf_cmap_mrange_count(fz_context *ctx, pdf_cmap *cmap);

/**
 * Check if CMap has a usecmap (parent).
 * @return 1 if has usecmap, 0 otherwise
 */
int pdf_cmap_has_usecmap(fz_context *ctx, pdf_cmap *cmap);

/**
 * Free a string allocated by CMap functions.
 */
void pdf_cmap_free_string(fz_context *ctx, char *s);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_CMAP_H */


