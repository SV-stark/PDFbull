/*
 * PDF Name Table FFI
 *
 * Provides PDF name string optimization through interning and
 * standard PDF name constants for efficient name comparisons.
 */

#ifndef MICROPDF_PDF_NAME_TABLE_H
#define MICROPDF_PDF_NAME_TABLE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Standard PDF Name Constants
 * ============================================================================ */

/* Document Structure */
#define PDF_NAME_TYPE           "Type"
#define PDF_NAME_SUBTYPE        "Subtype"
#define PDF_NAME_CATALOG        "Catalog"
#define PDF_NAME_PAGES          "Pages"
#define PDF_NAME_PAGE           "Page"
#define PDF_NAME_PARENT         "Parent"
#define PDF_NAME_KIDS           "Kids"
#define PDF_NAME_COUNT          "Count"
#define PDF_NAME_ROOT           "Root"
#define PDF_NAME_INFO           "Info"
#define PDF_NAME_METADATA       "Metadata"

/* Page Properties */
#define PDF_NAME_MEDIABOX       "MediaBox"
#define PDF_NAME_CROPBOX        "CropBox"
#define PDF_NAME_BLEEDBOX       "BleedBox"
#define PDF_NAME_TRIMBOX        "TrimBox"
#define PDF_NAME_ARTBOX         "ArtBox"
#define PDF_NAME_RESOURCES      "Resources"
#define PDF_NAME_CONTENTS       "Contents"
#define PDF_NAME_ROTATE         "Rotate"
#define PDF_NAME_USERUNIT       "UserUnit"

/* Resources */
#define PDF_NAME_EXTGSTATE      "ExtGState"
#define PDF_NAME_COLORSPACE     "ColorSpace"
#define PDF_NAME_PATTERN        "Pattern"
#define PDF_NAME_SHADING        "Shading"
#define PDF_NAME_XOBJECT        "XObject"
#define PDF_NAME_FONT           "Font"
#define PDF_NAME_PROCSET        "ProcSet"
#define PDF_NAME_PROPERTIES     "Properties"

/* XObject Types */
#define PDF_NAME_IMAGE          "Image"
#define PDF_NAME_FORM           "Form"

/* Stream Properties */
#define PDF_NAME_LENGTH         "Length"
#define PDF_NAME_FILTER         "Filter"
#define PDF_NAME_DECODEPARMS    "DecodeParms"

/* Filters */
#define PDF_NAME_ASCIIHEXDECODE   "ASCIIHexDecode"
#define PDF_NAME_ASCII85DECODE    "ASCII85Decode"
#define PDF_NAME_LZWDECODE        "LZWDecode"
#define PDF_NAME_FLATEDECODE      "FlateDecode"
#define PDF_NAME_RUNLENGTHDECODE  "RunLengthDecode"
#define PDF_NAME_CCITTFAXDECODE   "CCITTFaxDecode"
#define PDF_NAME_JBIG2DECODE      "JBIG2Decode"
#define PDF_NAME_DCTDECODE        "DCTDecode"
#define PDF_NAME_JPXDECODE        "JPXDecode"
#define PDF_NAME_CRYPT            "Crypt"
#define PDF_NAME_BROTLIDECODE     "BrotliDecode"

/* Color Spaces */
#define PDF_NAME_DEVICEGRAY     "DeviceGray"
#define PDF_NAME_DEVICERGB      "DeviceRGB"
#define PDF_NAME_DEVICECMYK     "DeviceCMYK"
#define PDF_NAME_CALGRAY        "CalGray"
#define PDF_NAME_CALRGB         "CalRGB"
#define PDF_NAME_LAB            "Lab"
#define PDF_NAME_ICCBASED       "ICCBased"
#define PDF_NAME_INDEXED        "Indexed"
#define PDF_NAME_SEPARATION     "Separation"
#define PDF_NAME_DEVICEN        "DeviceN"

/* Font Types */
#define PDF_NAME_TYPE0          "Type0"
#define PDF_NAME_TYPE1          "Type1"
#define PDF_NAME_MMTYPE1        "MMType1"
#define PDF_NAME_TYPE3          "Type3"
#define PDF_NAME_TRUETYPE       "TrueType"
#define PDF_NAME_CIDFONTTYPE0   "CIDFontType0"
#define PDF_NAME_CIDFONTTYPE2   "CIDFontType2"

/* Font Properties */
#define PDF_NAME_BASEFONT       "BaseFont"
#define PDF_NAME_ENCODING       "Encoding"
#define PDF_NAME_WIDTHS         "Widths"
#define PDF_NAME_FIRSTCHAR      "FirstChar"
#define PDF_NAME_LASTCHAR       "LastChar"

/* Image Properties */
#define PDF_NAME_WIDTH          "Width"
#define PDF_NAME_HEIGHT         "Height"
#define PDF_NAME_BITSPERCOMPONENT "BitsPerComponent"
#define PDF_NAME_IMAGEMASK      "ImageMask"
#define PDF_NAME_MASK           "Mask"
#define PDF_NAME_SMASK          "SMask"

/* Annotations */
#define PDF_NAME_ANNOT          "Annot"
#define PDF_NAME_ANNOTS         "Annots"
#define PDF_NAME_RECT           "Rect"
#define PDF_NAME_AP             "AP"

/* ============================================================================
 * Name Interning Functions
 * ============================================================================ */

/**
 * Intern a PDF name string, returning an index.
 * @param name The name string to intern
 * @return Index >= 0 on success, -1 on failure
 */
int pdf_intern_name(const char *name);

/**
 * Get an interned name by index.
 * @param idx The index returned by pdf_intern_name
 * @return Name string (caller must free with pdf_free_name_string) or NULL
 */
char *pdf_get_interned_name(int idx);

/**
 * Lookup a name index without interning.
 * @param name The name string to look up
 * @return Index >= 0 if found, -1 if not found
 */
int pdf_lookup_name(const char *name);

/**
 * Release a reference to an interned name.
 * @param idx The index to release
 */
void pdf_release_name(int idx);

/**
 * Compare two name indices for equality.
 * @return 1 if equal, 0 if not equal
 */
int pdf_name_index_eq(int a, int b);

/**
 * Compare a name index with a string.
 * @return 1 if equal, 0 if not equal
 */
int pdf_name_eq_str(int idx, const char *name);

/**
 * Free a name string returned by pdf_get_interned_name.
 */
void pdf_free_name_string(char *name);

/* ============================================================================
 * Standard Name Accessors
 * ============================================================================ */

/**
 * Get the index of the "Type" name.
 */
int pdf_std_name_type(void);

/**
 * Get the index of the "Subtype" name.
 */
int pdf_std_name_subtype(void);

/**
 * Get the index of the "Length" name.
 */
int pdf_std_name_length(void);

/**
 * Get the index of the "Filter" name.
 */
int pdf_std_name_filter(void);

/**
 * Get the index of the "Font" name.
 */
int pdf_std_name_font(void);

/**
 * Get the index of the "Image" name.
 */
int pdf_std_name_image(void);

/* ============================================================================
 * Statistics Functions
 * ============================================================================ */

/**
 * Get the number of interned names.
 */
int pdf_name_table_count(void);

/**
 * Get the total number of lookups.
 */
uint64_t pdf_name_table_lookups(void);

/**
 * Get the number of cache hits.
 */
uint64_t pdf_name_table_hits(void);

/**
 * Get the hit rate (0.0 - 1.0).
 */
double pdf_name_table_hit_rate(void);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_NAME_TABLE_H */


