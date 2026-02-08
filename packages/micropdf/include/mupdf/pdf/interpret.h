/*
 * PDF Content Stream Processor FFI
 *
 * Provides PDF content stream processing and interpretation capabilities.
 */

#ifndef MICROPDF_PDF_INTERPRET_H
#define MICROPDF_PDF_INTERPRET_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_processor;
typedef uint64_t pdf_obj;
typedef uint64_t fz_device;
typedef uint64_t fz_buffer;
typedef uint64_t fz_output;
typedef uint64_t pdf_annot;

/* Processor requirements */
typedef enum {
    PDF_PROCESSOR_NONE = 0,
    PDF_PROCESSOR_REQUIRES_DECODED_IMAGES = 1
} pdf_processor_requirements;

/* Processor types */
typedef enum {
    PDF_PROCESSOR_BASE = 0,
    PDF_PROCESSOR_RUN = 1,
    PDF_PROCESSOR_BUFFER = 2,
    PDF_PROCESSOR_OUTPUT = 3,
    PDF_PROCESSOR_SANITIZE = 4,
    PDF_PROCESSOR_COLOR = 5,
    PDF_PROCESSOR_VECTORIZE = 6
} pdf_processor_type;

/* Cull types */
typedef enum {
    FZ_CULL_PATH_DROP = 0,
    FZ_CULL_PATH_FILL = 1,
    FZ_CULL_PATH_STROKE = 2,
    FZ_CULL_PATH_FILL_STROKE = 3,
    FZ_CULL_CLIP_PATH_DROP = 4,
    FZ_CULL_CLIP_PATH_FILL = 5,
    FZ_CULL_CLIP_PATH_STROKE = 6,
    FZ_CULL_CLIP_PATH_FILL_STROKE = 7,
    FZ_CULL_GLYPH = 8,
    FZ_CULL_IMAGE = 9,
    FZ_CULL_SHADING = 10
} fz_cull_type;

/* ============================================================================
 * Processor Lifecycle
 * ============================================================================ */

/**
 * Create a new processor.
 * @param ctx Context handle
 * @param size Processor type (0=base, 1=run, 2=buffer, 3=output, 4=sanitize, 5=color, 6=vectorize)
 */
pdf_processor *pdf_new_processor(fz_context *ctx, int size);

/** Keep (increment ref count) a processor */
pdf_processor *pdf_keep_processor(fz_context *ctx, pdf_processor *proc);

/** Close a processor */
void pdf_close_processor(fz_context *ctx, pdf_processor *proc);

/** Drop a processor */
void pdf_drop_processor(fz_context *ctx, pdf_processor *proc);

/** Reset a processor for reuse */
void pdf_reset_processor(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * Processor Factories
 * ============================================================================ */

/** Create a run processor for rendering */
pdf_processor *pdf_new_run_processor(
    fz_context *ctx,
    pdf_document *doc,
    fz_device *dev,
    float a, float b, float c, float d, float e, float f,
    int struct_parent,
    const char *usage
);

/** Create a buffer processor */
pdf_processor *pdf_new_buffer_processor(
    fz_context *ctx,
    fz_buffer *buffer,
    int ahx_encode,
    int newlines
);

/** Create an output processor */
pdf_processor *pdf_new_output_processor(
    fz_context *ctx,
    fz_output *out,
    int ahx_encode,
    int newlines
);

/** Create a sanitize filter processor */
pdf_processor *pdf_new_sanitize_filter(
    fz_context *ctx,
    pdf_document *doc,
    pdf_processor *chain,
    int struct_parents,
    float a, float b, float c, float d, float e, float f
);

/** Create a color filter processor */
pdf_processor *pdf_new_color_filter(
    fz_context *ctx,
    pdf_document *doc,
    pdf_processor *chain,
    int struct_parents,
    float a, float b, float c, float d, float e, float f
);

/** Create a vectorize filter processor */
pdf_processor *pdf_new_vectorize_filter(
    fz_context *ctx,
    pdf_document *doc,
    pdf_processor *chain,
    int struct_parents,
    float a, float b, float c, float d, float e, float f
);

/* ============================================================================
 * Resource Stack
 * ============================================================================ */

/** Push resources onto processor stack */
void pdf_processor_push_resources(fz_context *ctx, pdf_processor *proc, pdf_obj *res);

/** Pop resources from processor stack */
pdf_obj *pdf_processor_pop_resources(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * Content Processing
 * ============================================================================ */

/** Process a content stream */
void pdf_process_contents(
    fz_context *ctx,
    pdf_processor *proc,
    pdf_document *doc,
    pdf_obj *res,
    pdf_obj *stm,
    pdf_obj **out_res
);

/** Process an annotation */
void pdf_process_annot(fz_context *ctx, pdf_processor *proc, pdf_annot *annot);

/** Process a glyph (for Type 3 fonts) */
void pdf_process_glyph(fz_context *ctx, pdf_processor *proc, pdf_document *doc, pdf_obj *res);

/** Process raw contents without resource handling */
void pdf_process_raw_contents(fz_context *ctx, pdf_processor *proc, pdf_document *doc, pdf_obj *stm);

/** Count q/Q balance in a content stream */
void pdf_count_q_balance(
    fz_context *ctx,
    pdf_document *doc,
    pdf_obj *res,
    pdf_obj *stm,
    int *prepend,
    int *append
);

/* ============================================================================
 * Graphics State Operators
 * ============================================================================ */

void pdf_op_w(fz_context *ctx, pdf_processor *proc, float linewidth);
void pdf_op_j(fz_context *ctx, pdf_processor *proc, int linejoin);
void pdf_op_J(fz_context *ctx, pdf_processor *proc, int linecap);
void pdf_op_M(fz_context *ctx, pdf_processor *proc, float miterlimit);
void pdf_op_d(fz_context *ctx, pdf_processor *proc, const float *array, int array_len, float phase);
void pdf_op_ri(fz_context *ctx, pdf_processor *proc, const char *intent);
void pdf_op_i(fz_context *ctx, pdf_processor *proc, float flatness);
void pdf_op_q(fz_context *ctx, pdf_processor *proc);
void pdf_op_Q(fz_context *ctx, pdf_processor *proc);
void pdf_op_cm(fz_context *ctx, pdf_processor *proc, float a, float b, float c, float d, float e, float f);

/* Extended graphics state */
void pdf_op_gs_begin(fz_context *ctx, pdf_processor *proc, const char *name);
void pdf_op_gs_BM(fz_context *ctx, pdf_processor *proc, const char *blendmode);
void pdf_op_gs_ca(fz_context *ctx, pdf_processor *proc, float alpha);
void pdf_op_gs_CA(fz_context *ctx, pdf_processor *proc, float alpha);
void pdf_op_gs_end(fz_context *ctx, pdf_processor *proc);
void pdf_op_gs_op(fz_context *ctx, pdf_processor *proc, int b);
void pdf_op_gs_OP(fz_context *ctx, pdf_processor *proc, int b);
void pdf_op_gs_OPM(fz_context *ctx, pdf_processor *proc, int i);

/* ============================================================================
 * Path Construction Operators
 * ============================================================================ */

void pdf_op_m(fz_context *ctx, pdf_processor *proc, float x, float y);
void pdf_op_l(fz_context *ctx, pdf_processor *proc, float x, float y);
void pdf_op_c(fz_context *ctx, pdf_processor *proc, float x1, float y1, float x2, float y2, float x3, float y3);
void pdf_op_v(fz_context *ctx, pdf_processor *proc, float x2, float y2, float x3, float y3);
void pdf_op_y(fz_context *ctx, pdf_processor *proc, float x1, float y1, float x3, float y3);
void pdf_op_h(fz_context *ctx, pdf_processor *proc);
void pdf_op_re(fz_context *ctx, pdf_processor *proc, float x, float y, float w, float h);

/* ============================================================================
 * Path Painting Operators
 * ============================================================================ */

void pdf_op_S(fz_context *ctx, pdf_processor *proc);
void pdf_op_s(fz_context *ctx, pdf_processor *proc);
void pdf_op_f(fz_context *ctx, pdf_processor *proc);
void pdf_op_F(fz_context *ctx, pdf_processor *proc);
void pdf_op_fstar(fz_context *ctx, pdf_processor *proc);
void pdf_op_B(fz_context *ctx, pdf_processor *proc);
void pdf_op_Bstar(fz_context *ctx, pdf_processor *proc);
void pdf_op_b(fz_context *ctx, pdf_processor *proc);
void pdf_op_bstar(fz_context *ctx, pdf_processor *proc);
void pdf_op_n(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * Clipping Operators
 * ============================================================================ */

void pdf_op_W(fz_context *ctx, pdf_processor *proc);
void pdf_op_Wstar(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * Text Object Operators
 * ============================================================================ */

void pdf_op_BT(fz_context *ctx, pdf_processor *proc);
void pdf_op_ET(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * Text State Operators
 * ============================================================================ */

void pdf_op_Tc(fz_context *ctx, pdf_processor *proc, float charspace);
void pdf_op_Tw(fz_context *ctx, pdf_processor *proc, float wordspace);
void pdf_op_Tz(fz_context *ctx, pdf_processor *proc, float scale);
void pdf_op_TL(fz_context *ctx, pdf_processor *proc, float leading);
void pdf_op_Tf(fz_context *ctx, pdf_processor *proc, const char *name, float size);
void pdf_op_Tr(fz_context *ctx, pdf_processor *proc, int render);
void pdf_op_Ts(fz_context *ctx, pdf_processor *proc, float rise);

/* ============================================================================
 * Text Positioning Operators
 * ============================================================================ */

void pdf_op_Td(fz_context *ctx, pdf_processor *proc, float tx, float ty);
void pdf_op_TD(fz_context *ctx, pdf_processor *proc, float tx, float ty);
void pdf_op_Tm(fz_context *ctx, pdf_processor *proc, float a, float b, float c, float d, float e, float f);
void pdf_op_Tstar(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * Text Showing Operators
 * ============================================================================ */

void pdf_op_Tj(fz_context *ctx, pdf_processor *proc, const char *str, size_t len);
void pdf_op_TJ(fz_context *ctx, pdf_processor *proc);
void pdf_op_squote(fz_context *ctx, pdf_processor *proc, const char *str, size_t len);
void pdf_op_dquote(fz_context *ctx, pdf_processor *proc, float aw, float ac, const char *str, size_t len);

/* ============================================================================
 * Type 3 Font Operators
 * ============================================================================ */

void pdf_op_d0(fz_context *ctx, pdf_processor *proc, float wx, float wy);
void pdf_op_d1(fz_context *ctx, pdf_processor *proc, float wx, float wy, float llx, float lly, float urx, float ury);

/* ============================================================================
 * Color Operators
 * ============================================================================ */

void pdf_op_CS(fz_context *ctx, pdf_processor *proc, const char *name);
void pdf_op_cs(fz_context *ctx, pdf_processor *proc, const char *name);
void pdf_op_SC_color(fz_context *ctx, pdf_processor *proc, int n, const float *color);
void pdf_op_sc_color(fz_context *ctx, pdf_processor *proc, int n, const float *color);
void pdf_op_G(fz_context *ctx, pdf_processor *proc, float g);
void pdf_op_g(fz_context *ctx, pdf_processor *proc, float g);
void pdf_op_RG(fz_context *ctx, pdf_processor *proc, float r, float g, float b);
void pdf_op_rg(fz_context *ctx, pdf_processor *proc, float r, float g, float b);
void pdf_op_K(fz_context *ctx, pdf_processor *proc, float c, float m, float y, float k);
void pdf_op_k(fz_context *ctx, pdf_processor *proc, float c, float m, float y, float k);

/* ============================================================================
 * XObject/Image/Shading Operators
 * ============================================================================ */

void pdf_op_BI(fz_context *ctx, pdf_processor *proc, uint64_t image);
void pdf_op_sh(fz_context *ctx, pdf_processor *proc, const char *name);
void pdf_op_Do_image(fz_context *ctx, pdf_processor *proc, const char *name, uint64_t image);
void pdf_op_Do_form(fz_context *ctx, pdf_processor *proc, const char *name, uint64_t form);

/* ============================================================================
 * Marked Content Operators
 * ============================================================================ */

void pdf_op_MP(fz_context *ctx, pdf_processor *proc, const char *tag);
void pdf_op_DP(fz_context *ctx, pdf_processor *proc, const char *tag);
void pdf_op_BMC(fz_context *ctx, pdf_processor *proc, const char *tag);
void pdf_op_BDC(fz_context *ctx, pdf_processor *proc, const char *tag);
void pdf_op_EMC(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * Compatibility Operators
 * ============================================================================ */

void pdf_op_BX(fz_context *ctx, pdf_processor *proc);
void pdf_op_EX(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * End Operators
 * ============================================================================ */

void pdf_op_EOD(fz_context *ctx, pdf_processor *proc);
void pdf_op_END(fz_context *ctx, pdf_processor *proc);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/** Get processor type */
int pdf_processor_get_type(fz_context *ctx, pdf_processor *proc);

/** Get number of operators processed */
int pdf_processor_get_operator_count(fz_context *ctx, pdf_processor *proc);

/** Get gstate stack depth */
int pdf_processor_get_gstate_depth(fz_context *ctx, pdf_processor *proc);

/** Check if processor is in text object */
int pdf_processor_in_text(fz_context *ctx, pdf_processor *proc);

/** Get current line width */
float pdf_processor_get_line_width(fz_context *ctx, pdf_processor *proc);

/** Get current CTM */
void pdf_processor_get_ctm(
    fz_context *ctx,
    pdf_processor *proc,
    float *a, float *b,
    float *c, float *d,
    float *e, float *f
);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_INTERPRET_H */

