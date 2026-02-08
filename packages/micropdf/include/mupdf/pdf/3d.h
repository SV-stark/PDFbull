/*
 * PDF 3D Annotation FFI
 *
 * Provides support for 3D annotations in PDF documents, including
 * U3D and PRC format streams, 3D views, and activation settings.
 */

#ifndef MICROPDF_PDF_3D_H
#define MICROPDF_PDF_3D_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_3d_annotation;

/* ============================================================================
 * 3D Stream Format Constants
 * ============================================================================ */

#define PDF_3D_FORMAT_U3D       0
#define PDF_3D_FORMAT_PRC       1
#define PDF_3D_FORMAT_UNKNOWN  -1

/* ============================================================================
 * 3D Activation Mode Constants
 * ============================================================================ */

#define PDF_3D_ACTIVATION_EXPLICIT      0
#define PDF_3D_ACTIVATION_PAGE_OPEN     1
#define PDF_3D_ACTIVATION_PAGE_VISIBLE  2

/* ============================================================================
 * 3D Deactivation Mode Constants
 * ============================================================================ */

#define PDF_3D_DEACTIVATION_EXPLICIT        0
#define PDF_3D_DEACTIVATION_PAGE_CLOSE      1
#define PDF_3D_DEACTIVATION_PAGE_INVISIBLE  2

/* ============================================================================
 * 3D Rendering Mode Constants
 * ============================================================================ */

#define PDF_3D_RENDER_SOLID                     0
#define PDF_3D_RENDER_SOLID_WIREFRAME           1
#define PDF_3D_RENDER_TRANSPARENT               2
#define PDF_3D_RENDER_TRANSPARENT_WIREFRAME     3
#define PDF_3D_RENDER_BOUNDING_BOX              4
#define PDF_3D_RENDER_TRANSPARENT_BBOX          5
#define PDF_3D_RENDER_TRANSPARENT_BBOX_OUTLINE  6
#define PDF_3D_RENDER_WIREFRAME                 7
#define PDF_3D_RENDER_SHADED_WIREFRAME          8
#define PDF_3D_RENDER_HIDDEN_WIREFRAME          9
#define PDF_3D_RENDER_VERTICES                  10
#define PDF_3D_RENDER_SHADED_VERTICES           11
#define PDF_3D_RENDER_ILLUSTRATION              12
#define PDF_3D_RENDER_SOLID_OUTLINE             13
#define PDF_3D_RENDER_SHADED_ILLUSTRATION       14

/* ============================================================================
 * 3D Lighting Scheme Constants
 * ============================================================================ */

#define PDF_3D_LIGHTING_ARTWORK   0
#define PDF_3D_LIGHTING_NONE      1
#define PDF_3D_LIGHTING_WHITE     2
#define PDF_3D_LIGHTING_DAY       3
#define PDF_3D_LIGHTING_NIGHT     4
#define PDF_3D_LIGHTING_HARD      5
#define PDF_3D_LIGHTING_PRIMARY   6
#define PDF_3D_LIGHTING_BLUE      7
#define PDF_3D_LIGHTING_RED       8
#define PDF_3D_LIGHTING_CUBE      9
#define PDF_3D_LIGHTING_CAD       10
#define PDF_3D_LIGHTING_HEADLAMP  11

/* ============================================================================
 * 3D Projection Type Constants
 * ============================================================================ */

#define PDF_3D_PROJECTION_PERSPECTIVE   0
#define PDF_3D_PROJECTION_ORTHOGRAPHIC  1

/* ============================================================================
 * Camera Structure
 * ============================================================================ */

/**
 * 3D camera/view position
 */
typedef struct {
    float pos_x;      /* Camera position X */
    float pos_y;      /* Camera position Y */
    float pos_z;      /* Camera position Z */
    float target_x;   /* Camera target X */
    float target_y;   /* Camera target Y */
    float target_z;   /* Camera target Z */
    float up_x;       /* Up vector X */
    float up_y;       /* Up vector Y */
    float up_z;       /* Up vector Z */
    float fov;        /* Field of view (degrees) */
    int projection;   /* Projection type */
} pdf_camera_3d;

/* ============================================================================
 * Annotation Management
 * ============================================================================ */

/**
 * Create a new 3D annotation data structure.
 */
pdf_3d_annotation *pdf_new_3d_annotation(fz_context *ctx);

/**
 * Drop a 3D annotation data structure.
 */
void pdf_drop_3d_annotation(fz_context *ctx, pdf_3d_annotation *annot);

/**
 * Set the 3D stream data (U3D format).
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_u3d_data(fz_context *ctx, pdf_3d_annotation *annot, const uint8_t *data, size_t len);

/**
 * Set the 3D stream data (PRC format).
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_prc_data(fz_context *ctx, pdf_3d_annotation *annot, const uint8_t *data, size_t len);

/**
 * Get the 3D stream format.
 * @return PDF_3D_FORMAT_* constant
 */
int pdf_3d_get_format(fz_context *ctx, pdf_3d_annotation *annot);

/**
 * Get the 3D stream data.
 * @param len_out Pointer to receive length
 * @return Pointer to data (owned by annotation) or NULL
 */
const uint8_t *pdf_3d_get_data(fz_context *ctx, pdf_3d_annotation *annot, size_t *len_out);

/* ============================================================================
 * View Management
 * ============================================================================ */

/**
 * Add a 3D view.
 * @param name View name
 * @return View index or -1 on error
 */
int pdf_3d_add_view(fz_context *ctx, pdf_3d_annotation *annot, const char *name);

/**
 * Get the number of views.
 */
int pdf_3d_view_count(fz_context *ctx, pdf_3d_annotation *annot);

/**
 * Get view name by index.
 * @return Name string (caller must free) or NULL
 */
char *pdf_3d_get_view_name(fz_context *ctx, pdf_3d_annotation *annot, int index);

/**
 * Set the default view.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_default_view(fz_context *ctx, pdf_3d_annotation *annot, int index);

/**
 * Get the default view index.
 */
int pdf_3d_get_default_view(fz_context *ctx, pdf_3d_annotation *annot);

/* ============================================================================
 * View Properties
 * ============================================================================ */

/**
 * Set view camera.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_view_camera(fz_context *ctx, pdf_3d_annotation *annot, int view_index, const pdf_camera_3d *camera);

/**
 * Get view camera.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_get_view_camera(fz_context *ctx, pdf_3d_annotation *annot, int view_index, pdf_camera_3d *camera_out);

/**
 * Set view render mode.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_view_render_mode(fz_context *ctx, pdf_3d_annotation *annot, int view_index, int mode);

/**
 * Set view lighting scheme.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_view_lighting(fz_context *ctx, pdf_3d_annotation *annot, int view_index, int lighting);

/**
 * Set view background color.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_view_background(fz_context *ctx, pdf_3d_annotation *annot, int view_index, float r, float g, float b, float a);

/* ============================================================================
 * Activation Settings
 * ============================================================================ */

/**
 * Set activation mode.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_activation(fz_context *ctx, pdf_3d_annotation *annot, int mode);

/**
 * Get activation mode.
 */
int pdf_3d_get_activation(fz_context *ctx, pdf_3d_annotation *annot);

/**
 * Set deactivation mode.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_deactivation(fz_context *ctx, pdf_3d_annotation *annot, int mode);

/**
 * Get deactivation mode.
 */
int pdf_3d_get_deactivation(fz_context *ctx, pdf_3d_annotation *annot);

/* ============================================================================
 * UI Settings
 * ============================================================================ */

/**
 * Set toolbar visibility.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_toolbar(fz_context *ctx, pdf_3d_annotation *annot, int show);

/**
 * Set navigation panel visibility.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_navigation(fz_context *ctx, pdf_3d_annotation *annot, int show);

/**
 * Set interactive mode.
 * @return 1 on success, 0 on failure
 */
int pdf_3d_set_interactive(fz_context *ctx, pdf_3d_annotation *annot, int interactive);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/**
 * Free a string returned by 3D functions.
 */
void pdf_3d_free_string(char *s);

/**
 * Get format name string.
 * @return Format name (caller must free) or NULL
 */
char *pdf_3d_format_to_string(fz_context *ctx, int format);

/**
 * Get render mode name string.
 * @return Mode name (caller must free) or NULL
 */
char *pdf_3d_render_mode_to_string(fz_context *ctx, int mode);

/**
 * Get lighting scheme name string.
 * @return Lighting name (caller must free) or NULL
 */
char *pdf_3d_lighting_to_string(fz_context *ctx, int lighting);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_3D_H */


