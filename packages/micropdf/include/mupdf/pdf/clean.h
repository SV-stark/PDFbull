/*
 * PDF Clean/Optimization FFI
 *
 * Provides PDF optimization, cleaning, linearization, and page rearrangement.
 */

#ifndef MICROPDF_PDF_CLEAN_H
#define MICROPDF_PDF_CLEAN_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t fz_output;
typedef uint64_t pdf_obj;

/* ============================================================================
 * Structure Options
 * ============================================================================ */

/** Structure tree handling options */
typedef enum {
    PDF_CLEAN_STRUCTURE_DROP = 0,   /* Remove structure tree */
    PDF_CLEAN_STRUCTURE_KEEP = 1    /* Preserve structure tree */
} pdf_clean_options_structure;

/** Vectorize options */
typedef enum {
    PDF_CLEAN_VECTORIZE_NO = 0,     /* Leave pages unchanged */
    PDF_CLEAN_VECTORIZE_YES = 1     /* Vectorize pages */
} pdf_clean_options_vectorize;

/* ============================================================================
 * Encryption Methods
 * ============================================================================ */

/** Encryption method */
typedef enum {
    PDF_ENCRYPT_KEEP = 0,       /* Keep existing encryption */
    PDF_ENCRYPT_NONE = 1,       /* Remove encryption */
    PDF_ENCRYPT_RC4_40 = 2,     /* RC4 40-bit */
    PDF_ENCRYPT_RC4_128 = 3,    /* RC4 128-bit */
    PDF_ENCRYPT_AES_128 = 4,    /* AES 128-bit */
    PDF_ENCRYPT_AES_256 = 5     /* AES 256-bit */
} pdf_encryption_method;

/* ============================================================================
 * Write Options
 * ============================================================================ */

/**
 * PDF write options.
 * Matches the command line options to 'mutool clean':
 *   g: garbage collect
 *   d, i, f: expand all, fonts, images
 *   l: linearize
 *   a: ascii hex encode
 *   z: deflate
 *   c: clean content streams
 *   s: sanitize content streams
 */
typedef struct {
    int do_incremental;         /* Write just changed objects */
    int do_pretty;              /* Pretty-print dictionaries/arrays */
    int do_ascii;               /* ASCII hex encode binary streams */
    int do_compress;            /* Compress streams (1=zlib, 2=brotli) */
    int do_compress_images;     /* Compress image streams */
    int do_compress_fonts;      /* Compress font streams */
    int do_decompress;          /* Decompress streams */
    int do_garbage;             /* Garbage collect (1=gc, 2=renumber, 3=dedupe) */
    int do_linear;              /* Write linearized */
    int do_clean;               /* Clean content streams */
    int do_sanitize;            /* Sanitize content streams */
    int do_appearance;          /* (Re)create appearance streams */
    int do_encrypt;             /* Encryption method */
    int dont_regenerate_id;     /* Don't regenerate ID */
    int permissions;            /* Document permissions */
    unsigned char opwd_utf8[128];   /* Owner password */
    unsigned char upwd_utf8[128];   /* User password */
    int do_snapshot;            /* Snapshot mode (internal) */
    int do_preserve_metadata;   /* Preserve metadata when cleaning */
    int do_use_objstms;         /* Use object streams */
    int compression_effort;     /* 0=default, 1=min, 100=max */
    int do_labels;              /* Add labels to objects */
} pdf_write_options;

/* ============================================================================
 * Image Rewriter Options
 * ============================================================================ */

/** Image rewriter options */
typedef struct {
    int color_depth;            /* Target color depth (0=keep) */
    int dpi;                    /* Target DPI (0=keep) */
    int jpeg_quality;           /* JPEG quality (0-100) */
    int recompress;             /* Recompress images */
} pdf_image_rewriter_options;

/* ============================================================================
 * Clean Options
 * ============================================================================ */

/** PDF clean options */
typedef struct {
    pdf_write_options write;
    pdf_image_rewriter_options image;
    int subset_fonts;
    pdf_clean_options_structure structure;
    pdf_clean_options_vectorize vectorize;
} pdf_clean_options;

/* ============================================================================
 * Default Options
 * ============================================================================ */

/**
 * Get default write options.
 */
pdf_write_options pdf_default_write_options(void);

/**
 * Get default clean options.
 */
pdf_clean_options pdf_default_clean_options(void);

/* ============================================================================
 * Option Parsing
 * ============================================================================ */

/**
 * Parse write options from string.
 * @param opts Options structure to fill
 * @param args Option string (e.g., "glzcs")
 * @return Pointer to opts
 */
pdf_write_options *pdf_parse_write_options(fz_context *ctx, pdf_write_options *opts, const char *args);

/**
 * Format write options to string.
 * @param buffer Output buffer
 * @param buffer_len Buffer size
 * @param opts Options to format
 * @return Pointer to buffer
 */
char *pdf_format_write_options(fz_context *ctx, char *buffer, size_t buffer_len, const pdf_write_options *opts);

/* ============================================================================
 * Document Operations
 * ============================================================================ */

/**
 * Check if document can be saved incrementally.
 */
int pdf_can_be_saved_incrementally(fz_context *ctx, pdf_document *doc);

/**
 * Check if document has unsaved signatures.
 */
int pdf_has_unsaved_sigs(fz_context *ctx, pdf_document *doc);

/**
 * Save document to file.
 */
void pdf_save_document(fz_context *ctx, pdf_document *doc, const char *filename, const pdf_write_options *opts);

/**
 * Write document to output stream.
 */
void pdf_write_document(fz_context *ctx, pdf_document *doc, fz_output *out, const pdf_write_options *opts);

/**
 * Save document snapshot.
 */
void pdf_save_snapshot(fz_context *ctx, pdf_document *doc, const char *filename);

/**
 * Write document snapshot to output stream.
 */
void pdf_write_snapshot(fz_context *ctx, pdf_document *doc, fz_output *out);

/**
 * Save document journal.
 */
void pdf_save_journal(fz_context *ctx, pdf_document *doc, const char *filename);

/**
 * Write document journal to output stream.
 */
void pdf_write_journal(fz_context *ctx, pdf_document *doc, fz_output *out);

/* ============================================================================
 * Clean Operations
 * ============================================================================ */

/**
 * Clean a PDF file.
 * @param infile Input file path
 * @param outfile Output file path
 * @param password Document password
 * @param opts Clean options
 * @param retainlen Number of objects to retain
 * @param retainlist List of objects to retain
 */
void pdf_clean_file(fz_context *ctx, const char *infile, const char *outfile, const char *password, const pdf_clean_options *opts, int retainlen, const char **retainlist);

/**
 * Rearrange pages in document.
 * @param count Number of pages in new order
 * @param pages Array of page indices
 * @param structure Structure tree handling
 */
void pdf_rearrange_pages(fz_context *ctx, pdf_document *doc, int count, const int *pages, pdf_clean_options_structure structure);

/**
 * Vectorize pages in document.
 * @param count Number of pages (0 for all)
 * @param pages Array of page indices
 * @param vectorize Vectorize option
 */
void pdf_vectorize_pages(fz_context *ctx, pdf_document *doc, int count, const int *pages, pdf_clean_options_vectorize vectorize);

/* ============================================================================
 * Object Operations
 * ============================================================================ */

/**
 * Clean a PDF object (remove unused entries).
 */
void pdf_clean_object_entries(fz_context *ctx, pdf_obj *obj);

/* ============================================================================
 * Optimization Helpers
 * ============================================================================ */

/**
 * Optimize PDF (convenience function).
 */
void pdf_optimize(fz_context *ctx, pdf_document *doc, const char *filename);

/**
 * Linearize PDF (convenience function).
 */
void pdf_linearize(fz_context *ctx, pdf_document *doc, const char *filename);

/**
 * Compress all streams in document.
 * @param method 1=zlib, 2=brotli
 */
void pdf_compress_streams(fz_context *ctx, pdf_document *doc, int method);

/**
 * Decompress all streams in document.
 */
void pdf_decompress_streams(fz_context *ctx, pdf_document *doc);

/**
 * Create object streams.
 */
void pdf_create_object_streams(fz_context *ctx, pdf_document *doc);

/**
 * Remove object streams.
 */
void pdf_remove_object_streams(fz_context *ctx, pdf_document *doc);

/**
 * Garbage collect unused objects.
 * @param level 1=collect, 2=renumber, 3=deduplicate
 */
void pdf_garbage_collect(fz_context *ctx, pdf_document *doc, int level);

/**
 * Deduplicate objects.
 */
void pdf_deduplicate_objects(fz_context *ctx, pdf_document *doc);

/**
 * Renumber objects.
 */
void pdf_renumber_objects(fz_context *ctx, pdf_document *doc);

/**
 * Remove unused resources.
 */
void pdf_remove_unused_resources(fz_context *ctx, pdf_document *doc);

/* ============================================================================
 * Encryption Functions
 * ============================================================================ */

/**
 * Set document encryption.
 * @param method Encryption method
 * @param permissions Permission flags
 * @param owner_pwd Owner password
 * @param user_pwd User password
 */
void pdf_set_encryption(fz_context *ctx, pdf_write_options *opts, int method, int permissions, const char *owner_pwd, const char *user_pwd);

/**
 * Remove document encryption.
 */
void pdf_remove_encryption(fz_context *ctx, pdf_write_options *opts);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/**
 * Free a string allocated by clean functions.
 */
void pdf_clean_free_string(fz_context *ctx, char *s);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_CLEAN_H */


