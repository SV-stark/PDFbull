/*
 * PDF Layer (Optional Content Groups) FFI
 *
 * Provides support for PDF Optional Content Groups (OCG) which allow
 * layers of content to be selectively shown or hidden.
 */

#ifndef MICROPDF_PDF_LAYER_H
#define MICROPDF_PDF_LAYER_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_ocg_descriptor;
typedef uint64_t pdf_obj;
typedef uint64_t pdf_resource_stack;

/* ============================================================================
 * Layer Config UI Types
 * ============================================================================ */

typedef enum {
    PDF_LAYER_UI_LABEL = 0,
    PDF_LAYER_UI_CHECKBOX = 1,
    PDF_LAYER_UI_RADIOBOX = 2
} pdf_layer_config_ui_type;

/* ============================================================================
 * Layer Configuration Structures
 * ============================================================================ */

/**
 * Layer configuration info.
 */
typedef struct {
    const char *name;
    const char *creator;
} pdf_layer_config;

/**
 * Layer config UI element info.
 */
typedef struct {
    const char *text;
    int depth;
    int ui_type;  /* pdf_layer_config_ui_type */
    int selected;
    int locked;
} pdf_layer_config_ui;

/* ============================================================================
 * Layer Count and Enumeration
 * ============================================================================ */

/**
 * Count the number of layer configurations.
 * @return Number of layer configurations
 */
int pdf_count_layer_configs(fz_context *ctx, pdf_document *doc);

/**
 * Count the number of layers (OCGs).
 * @return Number of layers
 */
int pdf_count_layers(fz_context *ctx, pdf_document *doc);

/**
 * Get layer name by index.
 * @param layer Layer index (0 to count-1)
 * @return Layer name (caller must free with pdf_layer_free_string)
 */
const char *pdf_layer_name(fz_context *ctx, pdf_document *doc, int layer);

/**
 * Check if a layer is enabled.
 * @param layer Layer index
 * @return 1 if enabled, 0 if disabled
 */
int pdf_layer_is_enabled(fz_context *ctx, pdf_document *doc, int layer);

/**
 * Enable or disable a layer.
 * @param layer Layer index
 * @param enabled 1 to enable, 0 to disable
 */
void pdf_enable_layer(fz_context *ctx, pdf_document *doc, int layer, int enabled);

/* ============================================================================
 * Layer Configuration Info
 * ============================================================================ */

/**
 * Get layer configuration info.
 * @param config_num Config index (0 to count-1)
 * @param info Pointer to structure to fill
 */
void pdf_layer_config_info(fz_context *ctx, pdf_document *doc, int config_num, pdf_layer_config *info);

/**
 * Get layer configuration creator.
 * @param config_num Config index
 * @return Creator string (caller must free with pdf_layer_free_string)
 */
const char *pdf_layer_config_creator(fz_context *ctx, pdf_document *doc, int config_num);

/**
 * Get layer configuration name.
 * @param config_num Config index
 * @return Name string (caller must free with pdf_layer_free_string)
 */
const char *pdf_layer_config_name(fz_context *ctx, pdf_document *doc, int config_num);

/**
 * Select a layer configuration.
 * Updates visibility of optional content groups.
 * @param config_num Config index to select
 */
void pdf_select_layer_config(fz_context *ctx, pdf_document *doc, int config_num);

/* ============================================================================
 * Layer Config UI
 * ============================================================================ */

/**
 * Count UI elements in current layer configuration.
 * @return Number of UI elements
 */
int pdf_count_layer_config_ui(fz_context *ctx, pdf_document *doc);

/**
 * Get layer config UI element info.
 * @param ui UI element index (0 to count-1)
 * @param info Pointer to structure to fill
 */
void pdf_layer_config_ui_info(fz_context *ctx, pdf_document *doc, int ui, pdf_layer_config_ui *info);

/**
 * Select a UI element (checkbox/radiobox).
 * Selecting a radiobox may deselect other radioboxes in the same group.
 * @param ui UI element index
 */
void pdf_select_layer_config_ui(fz_context *ctx, pdf_document *doc, int ui);

/**
 * Deselect a UI element.
 * @param ui UI element index
 */
void pdf_deselect_layer_config_ui(fz_context *ctx, pdf_document *doc, int ui);

/**
 * Toggle a UI element.
 * @param ui UI element index
 */
void pdf_toggle_layer_config_ui(fz_context *ctx, pdf_document *doc, int ui);

/* ============================================================================
 * UI Type Conversion
 * ============================================================================ */

/**
 * Convert UI type to string.
 * @param ui_type UI type value
 * @return Static string ("label", "checkbox", or "radiobox")
 */
const char *pdf_layer_config_ui_type_to_string(int ui_type);

/**
 * Convert string to UI type.
 * @param str Type name string
 * @return UI type value
 */
int pdf_layer_config_ui_type_from_string(const char *str);

/* ============================================================================
 * OCG Management
 * ============================================================================ */

/**
 * Read/create OCG descriptor for document.
 * @return OCG descriptor handle
 */
pdf_ocg_descriptor *pdf_read_ocg(fz_context *ctx, pdf_document *doc);

/**
 * Drop OCG descriptor for document.
 */
void pdf_drop_ocg(fz_context *ctx, pdf_document *doc);

/**
 * Check if an OCG is hidden.
 * @param rdb Resource database handle
 * @param usage Usage string (e.g., "View", "Print", "Export")
 * @param ocg OCG object handle
 * @return 1 if hidden, 0 if visible
 */
int pdf_is_ocg_hidden(fz_context *ctx, pdf_document *doc, pdf_resource_stack *rdb, const char *usage, pdf_obj *ocg);

/**
 * Set current layer configuration as the default.
 * Writes the current layer state back into the document.
 */
void pdf_set_layer_config_as_default(fz_context *ctx, pdf_document *doc);

/* ============================================================================
 * Layer Management (Additional)
 * ============================================================================ */

/**
 * Add a new layer to the document.
 * @param name Layer name
 * @param enabled Initial enabled state
 * @return Layer index, or -1 on error
 */
int pdf_add_layer(fz_context *ctx, pdf_document *doc, const char *name, int enabled);

/**
 * Add a layer configuration.
 * @param name Configuration name (may be NULL)
 * @param creator Configuration creator (may be NULL)
 * @return Config index, or -1 on error
 */
int pdf_add_layer_config(fz_context *ctx, pdf_document *doc, const char *name, const char *creator);

/**
 * Add a UI element to the current configuration.
 * @param text Display text (may be NULL)
 * @param depth Nesting depth in UI
 * @param ui_type UI element type (pdf_layer_config_ui_type)
 * @param selected Initial selected state
 * @param locked Whether the element is locked
 * @return UI element index, or -1 on error
 */
int pdf_add_layer_config_ui(fz_context *ctx, pdf_document *doc, const char *text, int depth, int ui_type, int selected, int locked);

/**
 * Check if OCG has unsaved changes.
 * @return 1 if modified, 0 otherwise
 */
int pdf_layer_has_unsaved_changes(fz_context *ctx, pdf_document *doc);

/**
 * Get current layer configuration index.
 * @return Current config index, or -1 if no OCG
 */
int pdf_get_current_layer_config(fz_context *ctx, pdf_document *doc);

/**
 * Free a string allocated by layer functions.
 */
void pdf_layer_free_string(fz_context *ctx, char *s);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_LAYER_H */


