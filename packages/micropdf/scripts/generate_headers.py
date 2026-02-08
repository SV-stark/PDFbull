#!/usr/bin/env python3
"""
Generate comprehensive MuPDF-compatible C headers from Rust FFI code.
This script extracts all #[unsafe(no_mangle)] function signatures and generates
complete C header files organized by module.
"""

import re
from pathlib import Path
from typing import Dict, List, Tuple
from collections import defaultdict

# Type mapping from Rust to C
TYPE_MAP = {
    'i32': 'int32_t',
    'u32': 'uint32_t',
    'i64': 'int64_t',
    'u64': 'uint64_t',
    'f32': 'float',
    'f64': 'double',
    'bool': 'bool',
    'usize': 'size_t',
    'isize': 'intptr_t',
    '()': 'void',
    'c_char': 'char',
    'c_int': 'int',
    'c_void': 'void',
    'c_float': 'float',
    'c_double': 'double',
}

def convert_rust_type_to_c(rust_type: str) -> str:
    """Convert a Rust type to its C equivalent."""
    rust_type = rust_type.strip()

    # Handle simple types
    if rust_type in TYPE_MAP:
        return TYPE_MAP[rust_type]

    # Handle pointers (including nested paths like std::ffi::c_char)
    # Process nested pointers recursively
    if rust_type.startswith('*const '):
        inner = rust_type[7:].strip()
        # Clean up fully qualified paths before recursing
        if '::' in inner and not inner.startswith('*'):
            inner = inner.split('::')[-1]
        # Map c_char to char before recursing
        if inner == 'c_char':
            inner = 'char'
        # Recursively convert the inner type (handles nested pointers)
        inner_c = convert_rust_type_to_c(inner)
        # Special case for char pointers
        if inner_c == 'char':
            return 'const char *'
        return f'{inner_c} const *'

    if rust_type.startswith('*mut '):
        inner = rust_type[5:].strip()
        # Recursively convert the inner type (handles nested pointers)
        inner_c = convert_rust_type_to_c(inner)
        return f'{inner_c} *'

    # Handle Handle types
    if 'Handle' in rust_type:
        return 'int32_t'

    # Handle struct types (fz_*, pdf_*)
    if rust_type.startswith(('fz_', 'pdf_')):
        return rust_type

    # Clean up fully qualified paths
    if '::' in rust_type:
        return rust_type.split('::')[-1]

    # Default: return as-is
    return rust_type

def extract_function_signature(lines: List[str], start_idx: int) -> Tuple[str, int]:
    """Extract a complete function signature starting from the given line."""
    sig_lines = []
    idx = start_idx
    paren_depth = 0
    found_fn = False

    while idx < len(lines):
        line = lines[idx].strip()

        # Skip empty lines and comments
        if not line or line.startswith('//'):
            idx += 1
            continue

        # Look for function declaration
        if 'pub' in line and 'extern' in line and 'fn ' in line:
            found_fn = True

        if found_fn:
            sig_lines.append(line)

            # Count parentheses
            paren_depth += line.count('(') - line.count(')')

            # Check if signature is complete
            if paren_depth == 0 and ('{'  in line or line.endswith(';')):
                break

            # Check if we've closed all parentheses
            if found_fn and paren_depth == 0 and '->' not in line:
                break

        idx += 1

    return ' '.join(sig_lines), idx

def parse_rust_ffi_function(rust_sig: str) -> Dict[str, str]:
    """Parse a Rust FFI function signature into components."""
    # Extract function name
    fn_match = re.search(r'fn\s+(\w+)', rust_sig)
    if not fn_match:
        return None

    fn_name = fn_match.group(1)

    # Extract parameters
    params_match = re.search(r'\((.*?)\)\s*(?:->|{|;)', rust_sig, re.DOTALL)
    if not params_match:
        return None

    params_str = params_match.group(1).strip()

    # Extract return type
    ret_match = re.search(r'->\s*([^{;]+)', rust_sig)
    ret_type = ret_match.group(1).strip() if ret_match else '()'

    # Parse parameters
    c_params = []
    if params_str:
        # Split by comma, but respect nested types
        params = []
        current = []
        depth = 0
        for char in params_str + ',':
            if char in '<([':
                depth += 1
            elif char in '>)]':
                depth -= 1
            elif char == ',' and depth == 0:
                params.append(''.join(current).strip())
                current = []
                continue
            current.append(char)

        for param in params:
            if not param:
                continue
            # Split parameter into name and type
            # Handle cases like "name: *const c_char" or "_ctx: i32"
            if ':' in param:
                parts = param.split(':', 1)
                if len(parts) == 2:
                    param_name = parts[0].strip()
                    param_type = parts[1].strip()
                    c_type = convert_rust_type_to_c(param_type)
                    c_params.append(f'{c_type} {param_name}')
            else:
                # Malformed parameter, skip it
                continue

    c_params_str = ', '.join(c_params) if c_params else 'void'
    c_ret_type = convert_rust_type_to_c(ret_type)

    return {
        'name': fn_name,
        'params': c_params_str,
        'return': c_ret_type,
        'declaration': f'{c_ret_type} {fn_name}({c_params_str});'
    }

def extract_ffi_functions(file_path: Path) -> List[Dict]:
    """Extract all FFI functions from a Rust source file."""
    with open(file_path, 'r') as f:
        lines = f.readlines()

    functions = []
    i = 0

    while i < len(lines):
        line = lines[i].strip()

        # Look for #[unsafe(no_mangle)] or #[no_mangle]
        if line in ['#[unsafe(no_mangle)]', '#[no_mangle]']:
            # Extract the function signature
            rust_sig, next_idx = extract_function_signature(lines, i + 1)

            if rust_sig:
                func_info = parse_rust_ffi_function(rust_sig)
                if func_info:
                    functions.append(func_info)

            i = next_idx

        i += 1

    return functions

def generate_module_header(module_name: str, functions: List[Dict], is_pdf: bool = False) -> str:
    """Generate a C header file for a module."""
    prefix = 'pdf' if is_pdf else 'fitz'
    guard = f'MUPDF_{prefix.upper()}_{module_name.upper()}_H'

    header = f"""// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: {module_name}

#ifndef {guard}
#define {guard}

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {{
#endif

// ============================================================================
// {module_name.capitalize()} Functions ({len(functions)} total)
// ============================================================================

"""

    # Add function declarations
    for func in sorted(functions, key=lambda f: f['name']):
        header += func['declaration'] + '\n'

    header += f"""
#ifdef __cplusplus
}}
#endif

#endif /* {guard} */
"""

    return header

def generate_enhanced_header(module_name: str, functions: List[Dict]) -> str:
    """Generate a C header file for the enhanced (MicroPDF-specific) module."""
    guard = f'MICROPDF_{module_name.upper()}_H'

    header = f"""// MicroPDF - Enhanced/Extended Functions
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: {module_name}
//
// These are MicroPDF-specific extensions beyond MuPDF compatibility.
// All functions are prefixed with mp_* to distinguish from MuPDF functions.

#ifndef {guard}
#define {guard}

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {{
#endif

// ============================================================================
// {module_name.capitalize()} Functions ({len(functions)} total)
// ============================================================================

"""

    # Add function declarations
    for func in sorted(functions, key=lambda f: f['name']):
        header += func['declaration'] + '\n'

    header += f"""
#ifdef __cplusplus
}}
#endif

#endif /* {guard} */
"""

    return header

def main():
    """Main entry point."""
    src_dir = Path('src/ffi')
    include_dir = Path('include/mupdf')
    include_dir.mkdir(parents=True, exist_ok=True)

    # Create subdirectories
    (include_dir / 'fitz').mkdir(exist_ok=True)
    (include_dir / 'pdf').mkdir(exist_ok=True)
    Path('include/micropdf').mkdir(parents=True, exist_ok=True)

    # Module categorization
    fitz_modules = {
        'geometry', 'buffer', 'stream', 'output', 'colorspace',
        'pixmap', 'font', 'image', 'path', 'text', 'device',
        'display_list', 'link', 'archive', 'cookie', 'context'
    }

    pdf_modules = {'annot', 'form', 'document', 'pdf_object'}

    all_functions = defaultdict(list)
    total_functions = 0

    print("Extracting FFI functions from Rust source...")

    # Process each FFI module (both files and directories)
    for rs_file in sorted(src_dir.glob('*.rs')):
        module = rs_file.stem
        if module in ['mod', 'safe_helpers']:
            continue

        functions = extract_ffi_functions(rs_file)
        if functions:
            all_functions[module] = functions
            total_functions += len(functions)
            print(f"  {module:20s}: {len(functions):3d} functions")

    # Process subdirectories (like pdf_object)
    for sub_dir in src_dir.iterdir():
        if sub_dir.is_dir():
            module = sub_dir.name
            functions = []
            for rs_file in sorted(sub_dir.glob('**/*.rs')):
                funcs = extract_ffi_functions(rs_file)
                functions.extend(funcs)

            if functions:
                all_functions[module] = functions
                total_functions += len(functions)
                print(f"  {module:20s}: {len(functions):3d} functions")

    print(f"\nTotal: {total_functions} functions across {len(all_functions)} modules\n")

    # Generate headers
    print("Generating header files...")

    for module, functions in all_functions.items():
        # Enhanced module goes in micropdf/ directory, not mupdf/
        if module == 'enhanced':
            subdir = '../micropdf'
            header_content = generate_enhanced_header(module, functions)
        else:
            is_pdf = module in pdf_modules or any(f['name'].startswith('pdf_') for f in functions)
            subdir = 'pdf' if is_pdf else 'fitz'
            header_content = generate_module_header(module, functions, is_pdf)

        header_file = include_dir / subdir / f'{module}.h'

        with open(header_file, 'w') as f:
            f.write(header_content)

        print(f"  Generated: {header_file}")

    # Generate master headers
    generate_master_headers(include_dir, list(all_functions.keys()), fitz_modules, pdf_modules)

    print(f"\n✅ Successfully generated headers for {total_functions} functions!")

def generate_master_headers(include_dir: Path, all_modules: List[str], fitz_modules: set, pdf_modules: set):
    """Generate master include headers (fitz.h, pdf.h, mupdf.h)."""

    # Generate fitz.h
    fitz_header = """// MicroPDF - MuPDF API Compatible C Header
// Master header for fitz (core) functionality

#ifndef MUPDF_FITZ_H
#define MUPDF_FITZ_H

#ifdef __cplusplus
extern "C" {
#endif

"""

    for module in sorted(all_modules):
        # Skip enhanced module - it's in micropdf/ not mupdf/fitz/
        if module == 'enhanced':
            continue
        if module in fitz_modules or not any(module in pdf_modules for _ in [1]):
            fitz_header += f'#include "mupdf/fitz/{module}.h"\n'

    fitz_header += """
#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_H */
"""

    with open(include_dir / 'fitz.h', 'w') as f:
        f.write(fitz_header)
    print(f"  Generated: {include_dir / 'fitz.h'}")

    # Generate pdf.h
    pdf_header = """// MicroPDF - MuPDF API Compatible C Header
// Master header for PDF-specific functionality

#ifndef MUPDF_PDF_H
#define MUPDF_PDF_H

#include "mupdf/fitz.h"

#ifdef __cplusplus
extern "C" {
#endif

"""

    for module in sorted(all_modules):
        if module in pdf_modules:
            pdf_header += f'#include "mupdf/pdf/{module}.h"\n'

    pdf_header += """
#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_H */
"""

    with open(include_dir / 'pdf.h', 'w') as f:
        f.write(pdf_header)
    print(f"  Generated: {include_dir / 'pdf.h'}")

    # Generate mupdf.h
    mupdf_header = """// MicroPDF - MuPDF API Compatible C Header
// Single include header for all MuPDF functionality

#ifndef MUPDF_H
#define MUPDF_H

#include "mupdf/fitz.h"
#include "mupdf/pdf.h"

#endif /* MUPDF_H */
"""

    with open(Path('include') / 'mupdf.h', 'w') as f:
        f.write(mupdf_header)
    print(f"  Generated: include/mupdf.h")

if __name__ == '__main__':
    main()

