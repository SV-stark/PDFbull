// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_interpret

#ifndef MUPDF_PDF_PDF_INTERPRET_H
#define MUPDF_PDF_PDF_INTERPRET_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_interpret Functions (103 total)
// ============================================================================

void pdf_close_processor(int32_t _ctx, int32_t proc);
void pdf_count_q_balance(int32_t _ctx, int32_t _doc, int32_t _res, int32_t _stm, int32_t * prepend, int32_t * append);
void pdf_drop_processor(int32_t _ctx, int32_t proc);
int32_t pdf_keep_processor(int32_t _ctx, int32_t proc);
int32_t pdf_new_buffer_processor(int32_t _ctx, int32_t buffer, int32_t ahx_encode, int32_t newlines);
int32_t pdf_new_color_filter(int32_t _ctx, int32_t _doc, int32_t chain, int32_t struct_parents, float a, float b, float c, float d, float e, float f);
int32_t pdf_new_output_processor(int32_t _ctx, int32_t out, int32_t ahx_encode, int32_t newlines);
int32_t pdf_new_processor(int32_t _ctx, int32_t size);
int32_t pdf_new_run_processor(int32_t _ctx, int32_t doc, int32_t dev, float a, float b, float c, float d, float e, float f, int32_t struct_parent, const char * usage);
int32_t pdf_new_sanitize_filter(int32_t _ctx, int32_t _doc, int32_t chain, int32_t struct_parents, float a, float b, float c, float d, float e, float f);
int32_t pdf_new_vectorize_filter(int32_t _ctx, int32_t _doc, int32_t chain, int32_t struct_parents, float a, float b, float c, float d, float e, float f);
void pdf_op_B(int32_t _ctx, int32_t proc);
void pdf_op_BDC(int32_t _ctx, int32_t proc, const char * tag);
void pdf_op_BI(int32_t _ctx, int32_t proc, int32_t image);
void pdf_op_BMC(int32_t _ctx, int32_t proc, const char * tag);
void pdf_op_BT(int32_t _ctx, int32_t proc);
void pdf_op_BX(int32_t _ctx, int32_t proc);
void pdf_op_Bstar(int32_t _ctx, int32_t proc);
void pdf_op_CS(int32_t _ctx, int32_t proc, const char * name);
void pdf_op_DP(int32_t _ctx, int32_t proc, const char * tag);
void pdf_op_Do_form(int32_t _ctx, int32_t proc, const char * name, int32_t form);
void pdf_op_Do_image(int32_t _ctx, int32_t proc, const char * name, int32_t image);
void pdf_op_EMC(int32_t _ctx, int32_t proc);
void pdf_op_END(int32_t _ctx, int32_t proc);
void pdf_op_EOD(int32_t _ctx, int32_t proc);
void pdf_op_ET(int32_t _ctx, int32_t proc);
void pdf_op_EX(int32_t _ctx, int32_t proc);
void pdf_op_F(int32_t _ctx, int32_t proc);
void pdf_op_G(int32_t _ctx, int32_t proc, float g);
void pdf_op_J(int32_t _ctx, int32_t proc, int32_t linecap);
void pdf_op_K(int32_t _ctx, int32_t proc, float c, float m, float y, float k);
void pdf_op_M(int32_t _ctx, int32_t proc, float miterlimit);
void pdf_op_MP(int32_t _ctx, int32_t proc, const char * tag);
void pdf_op_Q(int32_t _ctx, int32_t proc);
void pdf_op_RG(int32_t _ctx, int32_t proc, float r, float g, float b);
void pdf_op_S(int32_t _ctx, int32_t proc);
void pdf_op_SC_color(int32_t _ctx, int32_t proc, int32_t n, float const * color);
void pdf_op_TD(int32_t _ctx, int32_t proc, float tx, float ty);
void pdf_op_TJ(int32_t _ctx, int32_t proc);
void pdf_op_TL(int32_t _ctx, int32_t proc, float leading);
void pdf_op_Tc(int32_t _ctx, int32_t proc, float charspace);
void pdf_op_Td(int32_t _ctx, int32_t proc, float tx, float ty);
void pdf_op_Tf(int32_t _ctx, int32_t proc, const char * name, float size);
void pdf_op_Tj(int32_t _ctx, int32_t proc, const char * str, size_t len);
void pdf_op_Tm(int32_t _ctx, int32_t proc, float a, float b, float c, float d, float e, float f);
void pdf_op_Tr(int32_t _ctx, int32_t proc, int32_t render);
void pdf_op_Ts(int32_t _ctx, int32_t proc, float rise);
void pdf_op_Tstar(int32_t _ctx, int32_t proc);
void pdf_op_Tw(int32_t _ctx, int32_t proc, float wordspace);
void pdf_op_Tz(int32_t _ctx, int32_t proc, float scale);
void pdf_op_W(int32_t _ctx, int32_t proc);
void pdf_op_Wstar(int32_t _ctx, int32_t proc);
void pdf_op_b(int32_t _ctx, int32_t proc);
void pdf_op_bstar(int32_t _ctx, int32_t proc);
void pdf_op_c(int32_t _ctx, int32_t proc, float x1, float y1, float x2, float y2, float x3, float y3);
void pdf_op_cm(int32_t _ctx, int32_t proc, float a, float b, float c, float d, float e, float f);
void pdf_op_cs(int32_t _ctx, int32_t proc, const char * name);
void pdf_op_d(int32_t _ctx, int32_t proc, float const * array, int32_t array_len, float phase);
void pdf_op_d0(int32_t _ctx, int32_t proc, float wx, float wy);
void pdf_op_d1(int32_t _ctx, int32_t proc, float wx, float wy, float llx, float lly, float urx, float ury);
void pdf_op_dquote(int32_t _ctx, int32_t proc, float aw, float ac, const char * str, size_t len);
void pdf_op_f(int32_t _ctx, int32_t proc);
void pdf_op_fstar(int32_t _ctx, int32_t proc);
void pdf_op_g(int32_t _ctx, int32_t proc, float g);
void pdf_op_gs_BM(int32_t _ctx, int32_t proc, const char * blendmode);
void pdf_op_gs_CA(int32_t _ctx, int32_t proc, float alpha);
void pdf_op_gs_OP(int32_t _ctx, int32_t proc, int32_t b);
void pdf_op_gs_OPM(int32_t _ctx, int32_t proc, int32_t i);
void pdf_op_gs_begin(int32_t _ctx, int32_t proc, const char * name);
void pdf_op_gs_ca(int32_t _ctx, int32_t proc, float alpha);
void pdf_op_gs_end(int32_t _ctx, int32_t _proc);
void pdf_op_gs_op(int32_t _ctx, int32_t proc, int32_t b);
void pdf_op_h(int32_t _ctx, int32_t proc);
void pdf_op_i(int32_t _ctx, int32_t proc, float flatness);
void pdf_op_j(int32_t _ctx, int32_t proc, int32_t linejoin);
void pdf_op_k(int32_t _ctx, int32_t proc, float c, float m, float y, float k);
void pdf_op_l(int32_t _ctx, int32_t proc, float x, float y);
void pdf_op_m(int32_t _ctx, int32_t proc, float x, float y);
void pdf_op_n(int32_t _ctx, int32_t proc);
void pdf_op_q(int32_t _ctx, int32_t proc);
void pdf_op_re(int32_t _ctx, int32_t proc, float x, float y, float w, float h);
void pdf_op_rg(int32_t _ctx, int32_t proc, float r, float g, float b);
void pdf_op_ri(int32_t _ctx, int32_t proc, const char * intent);
void pdf_op_s(int32_t _ctx, int32_t proc);
void pdf_op_sc_color(int32_t _ctx, int32_t proc, int32_t n, float const * color);
void pdf_op_sh(int32_t _ctx, int32_t proc, const char * name);
void pdf_op_squote(int32_t _ctx, int32_t proc, const char * str, size_t len);
void pdf_op_v(int32_t _ctx, int32_t proc, float x2, float y2, float x3, float y3);
void pdf_op_w(int32_t _ctx, int32_t proc, float linewidth);
void pdf_op_y(int32_t _ctx, int32_t proc, float x1, float y1, float x3, float y3);
void pdf_process_annot(int32_t _ctx, int32_t proc, int32_t annot);
void pdf_process_contents(int32_t _ctx, int32_t proc, int32_t _doc, int32_t res, int32_t _stm, int32_t * _out_res);
void pdf_process_glyph(int32_t _ctx, int32_t proc, int32_t _doc, int32_t res);
void pdf_process_raw_contents(int32_t _ctx, int32_t proc, int32_t _doc, int32_t _stm);
void pdf_processor_get_ctm(int32_t _ctx, int32_t proc, float * a, float * b, float * c, float * d, float * e, float * f);
int32_t pdf_processor_get_gstate_depth(int32_t _ctx, int32_t proc);
float pdf_processor_get_line_width(int32_t _ctx, int32_t proc);
int32_t pdf_processor_get_operator_count(int32_t _ctx, int32_t proc);
int32_t pdf_processor_get_type(int32_t _ctx, int32_t proc);
int32_t pdf_processor_in_text(int32_t _ctx, int32_t proc);
int32_t pdf_processor_pop_resources(int32_t _ctx, int32_t proc);
void pdf_processor_push_resources(int32_t _ctx, int32_t proc, int32_t res);
void pdf_reset_processor(int32_t _ctx, int32_t proc);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_INTERPRET_H */
