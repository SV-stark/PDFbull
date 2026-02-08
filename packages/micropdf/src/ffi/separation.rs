//! C FFI for separation/spot colors - MuPDF compatible
//! Safe Rust implementation of fz_separation

use super::{Handle, HandleStore};
use std::ffi::{CStr, c_char};
use std::sync::LazyLock;

/// Maximum number of separations supported
pub const FZ_MAX_SEPARATIONS: usize = 64;

/// Separation behavior enumeration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparationBehavior {
    /// Composite (default rendering)
    Composite = 0,
    /// Spot color (preserve separation)
    Spot = 1,
    /// Disabled (don't render)
    Disabled = 2,
}

/// Individual separation definition
#[derive(Debug, Clone)]
pub struct Separation {
    /// Name of the separation (e.g., "PANTONE 185 C")
    pub name: String,
    /// Equivalent CMYK values for proofing/fallback
    pub cmyk: [f32; 4],
    /// Current behavior
    pub behavior: SeparationBehavior,
    /// Whether this is an "All" separation
    pub is_all: bool,
    /// Whether this is a "None" separation  
    pub is_none: bool,
}

impl Default for Separation {
    fn default() -> Self {
        Self {
            name: String::new(),
            cmyk: [0.0, 0.0, 0.0, 0.0],
            behavior: SeparationBehavior::Composite,
            is_all: false,
            is_none: false,
        }
    }
}

/// Separations collection
#[derive(Debug, Clone)]
pub struct Separations {
    /// List of separations
    pub seps: Vec<Separation>,
    /// Whether controllable (can change behavior)
    pub controllable: bool,
}

impl Default for Separations {
    fn default() -> Self {
        Self {
            seps: Vec::new(),
            controllable: true,
        }
    }
}

/// Global separations storage
pub static SEPARATIONS: LazyLock<HandleStore<Separations>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Separations Creation
// ============================================================================

/// Create a new separations object
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_separations(_ctx: Handle, controllable: i32) -> Handle {
    let seps = Separations {
        controllable: controllable != 0,
        ..Default::default()
    };
    SEPARATIONS.insert(seps)
}

/// Add a separation to the collection
///
/// # Safety
/// `name` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_add_separation(
    _ctx: Handle,
    seps: Handle,
    name: *const c_char,
    _colorspace: u64,
    cmyk_c: f32,
    cmyk_m: f32,
    cmyk_y: f32,
    cmyk_k: f32,
) -> i32 {
    let sep_name = if name.is_null() {
        String::new()
    } else {
        let c_str = unsafe { CStr::from_ptr(name) };
        c_str.to_str().unwrap_or("").to_string()
    };

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(mut guard) = seps_arc.lock() {
            if guard.seps.len() >= FZ_MAX_SEPARATIONS {
                return -1;
            }

            let index = guard.seps.len() as i32;
            let sep = Separation {
                name: sep_name,
                cmyk: [
                    cmyk_c.clamp(0.0, 1.0),
                    cmyk_m.clamp(0.0, 1.0),
                    cmyk_y.clamp(0.0, 1.0),
                    cmyk_k.clamp(0.0, 1.0),
                ],
                behavior: SeparationBehavior::Composite,
                is_all: false,
                is_none: false,
            };
            guard.seps.push(sep);
            return index;
        }
    }
    -1
}

/// Add a special "All" separation
#[unsafe(no_mangle)]
pub extern "C" fn fz_add_separation_all(_ctx: Handle, seps: Handle) -> i32 {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(mut guard) = seps_arc.lock() {
            if guard.seps.len() >= FZ_MAX_SEPARATIONS {
                return -1;
            }

            let index = guard.seps.len() as i32;
            let sep = Separation {
                name: "All".to_string(),
                cmyk: [0.0, 0.0, 0.0, 1.0],
                behavior: SeparationBehavior::Composite,
                is_all: true,
                is_none: false,
            };
            guard.seps.push(sep);
            return index;
        }
    }
    -1
}

/// Add a special "None" separation
#[unsafe(no_mangle)]
pub extern "C" fn fz_add_separation_none(_ctx: Handle, seps: Handle) -> i32 {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(mut guard) = seps_arc.lock() {
            if guard.seps.len() >= FZ_MAX_SEPARATIONS {
                return -1;
            }

            let index = guard.seps.len() as i32;
            let sep = Separation {
                name: "None".to_string(),
                cmyk: [0.0, 0.0, 0.0, 0.0],
                behavior: SeparationBehavior::Disabled,
                is_all: false,
                is_none: true,
            };
            guard.seps.push(sep);
            return index;
        }
    }
    -1
}

// ============================================================================
// Separations Query
// ============================================================================

/// Get number of separations
#[unsafe(no_mangle)]
pub extern "C" fn fz_count_separations(_ctx: Handle, seps: Handle) -> i32 {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            return guard.seps.len() as i32;
        }
    }
    0
}

/// Get separation name
///
/// Returns a pointer to static storage; caller must not free.
#[unsafe(no_mangle)]
pub extern "C" fn fz_separation_name(_ctx: Handle, seps: Handle, idx: i32) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if idx < 0 {
        return EMPTY.as_ptr().cast();
    }

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            if let Some(sep) = guard.seps.get(idx as usize) {
                // Return pointer to internal string data
                // This is safe because the string is owned by the HandleStore
                return sep.name.as_ptr().cast();
            }
        }
    }
    EMPTY.as_ptr().cast()
}

/// Check if separation is controllable
#[unsafe(no_mangle)]
pub extern "C" fn fz_separations_controllable(_ctx: Handle, seps: Handle) -> i32 {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            return i32::from(guard.controllable);
        }
    }
    0
}

/// Get separation behavior
#[unsafe(no_mangle)]
pub extern "C" fn fz_separation_current_behavior(_ctx: Handle, seps: Handle, idx: i32) -> i32 {
    if idx < 0 {
        return SeparationBehavior::Disabled as i32;
    }

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            if let Some(sep) = guard.seps.get(idx as usize) {
                return sep.behavior as i32;
            }
        }
    }
    SeparationBehavior::Disabled as i32
}

/// Set separation behavior
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_separation_behavior(_ctx: Handle, seps: Handle, idx: i32, behavior: i32) {
    if idx < 0 {
        return;
    }

    let b = match behavior {
        0 => SeparationBehavior::Composite,
        1 => SeparationBehavior::Spot,
        _ => SeparationBehavior::Disabled,
    };

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(mut guard) = seps_arc.lock() {
            if !guard.controllable {
                return;
            }
            if let Some(sep) = guard.seps.get_mut(idx as usize) {
                sep.behavior = b;
            }
        }
    }
}

/// Check if separation is "All"
#[unsafe(no_mangle)]
pub extern "C" fn fz_separation_is_all(_ctx: Handle, seps: Handle, idx: i32) -> i32 {
    if idx < 0 {
        return 0;
    }

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            if let Some(sep) = guard.seps.get(idx as usize) {
                return i32::from(sep.is_all);
            }
        }
    }
    0
}

/// Check if separation is "None"
#[unsafe(no_mangle)]
pub extern "C" fn fz_separation_is_none(_ctx: Handle, seps: Handle, idx: i32) -> i32 {
    if idx < 0 {
        return 0;
    }

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            if let Some(sep) = guard.seps.get(idx as usize) {
                return i32::from(sep.is_none);
            }
        }
    }
    0
}

// ============================================================================
// Separation Color Conversion
// ============================================================================

/// Get CMYK equivalent for separation
///
/// # Safety
/// `cmyk` must point to at least 4 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_separation_equivalent(_ctx: Handle, seps: Handle, idx: i32, cmyk: *mut f32) {
    if cmyk.is_null() || idx < 0 {
        return;
    }

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            if let Some(sep) = guard.seps.get(idx as usize) {
                let cmyk_slice = unsafe { std::slice::from_raw_parts_mut(cmyk, 4) };
                cmyk_slice.copy_from_slice(&sep.cmyk);
            }
        }
    }
}

/// Set CMYK equivalent for separation
///
/// # Safety
/// `cmyk` must point to at least 4 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_separation_equivalent(
    _ctx: Handle,
    seps: Handle,
    idx: i32,
    cmyk: *const f32,
) {
    if cmyk.is_null() || idx < 0 {
        return;
    }

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(mut guard) = seps_arc.lock() {
            if let Some(sep) = guard.seps.get_mut(idx as usize) {
                let cmyk_slice = unsafe { std::slice::from_raw_parts(cmyk, 4) };
                sep.cmyk.copy_from_slice(cmyk_slice);
            }
        }
    }
}

/// Convert separation color to destination colorspace
///
/// # Safety
/// - `src` must point to `src_n` floats
/// - `dst` must point to `dst_n` floats
#[unsafe(no_mangle)]
pub extern "C" fn fz_convert_separation_colors(
    _ctx: Handle,
    seps: Handle,
    src: *const f32,
    src_n: i32,
    dst: *mut f32,
    dst_n: i32,
) {
    if src.is_null() || dst.is_null() || src_n <= 0 || dst_n <= 0 {
        return;
    }

    let src_slice = unsafe { std::slice::from_raw_parts(src, src_n as usize) };
    let dst_slice = unsafe { std::slice::from_raw_parts_mut(dst, dst_n as usize) };

    // Initialize destination to 0
    dst_slice.fill(0.0);

    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            // Composite all active separations
            for (i, sep) in guard.seps.iter().enumerate() {
                if i >= src_n as usize {
                    break;
                }

                if sep.behavior == SeparationBehavior::Disabled {
                    continue;
                }

                let intensity = src_slice[i];
                if intensity <= 0.0 {
                    continue;
                }

                // Add this separation's CMYK contribution
                if dst_n >= 4 {
                    // Output is CMYK
                    for j in 0..4 {
                        dst_slice[j] = (dst_slice[j] + sep.cmyk[j] * intensity).min(1.0);
                    }
                } else if dst_n >= 3 {
                    // Output is RGB - convert CMYK to RGB
                    let c = sep.cmyk[0] * intensity;
                    let m = sep.cmyk[1] * intensity;
                    let y = sep.cmyk[2] * intensity;
                    let k = sep.cmyk[3] * intensity;

                    dst_slice[0] = (1.0 - dst_slice[0] - (1.0 - c) * (1.0 - k)).max(0.0);
                    dst_slice[1] = (1.0 - dst_slice[1] - (1.0 - m) * (1.0 - k)).max(0.0);
                    dst_slice[2] = (1.0 - dst_slice[2] - (1.0 - y) * (1.0 - k)).max(0.0);
                }
            }
        }
    }
}

// ============================================================================
// Copy/Clone Separations
// ============================================================================

/// Clone separations
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_separations(_ctx: Handle, seps: Handle) -> Handle {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            let cloned = guard.clone();
            return SEPARATIONS.insert(cloned);
        }
    }
    0
}

/// Compare two separations objects for equality
#[unsafe(no_mangle)]
pub extern "C" fn fz_separations_equal(_ctx: Handle, seps1: Handle, seps2: Handle) -> i32 {
    if seps1 == seps2 {
        return 1;
    }

    let s1 = SEPARATIONS.get(seps1);
    let s2 = SEPARATIONS.get(seps2);

    match (s1, s2) {
        (Some(arc1), Some(arc2)) => {
            let (g1, g2) = match (arc1.lock(), arc2.lock()) {
                (Ok(g1), Ok(g2)) => (g1, g2),
                _ => return 0,
            };

            if g1.seps.len() != g2.seps.len() {
                return 0;
            }

            for (s1, s2) in g1.seps.iter().zip(g2.seps.iter()) {
                if s1.name != s2.name || s1.cmyk != s2.cmyk {
                    return 0;
                }
            }
            1
        }
        _ => 0,
    }
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Keep separations reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_separations(_ctx: Handle, seps: Handle) -> Handle {
    SEPARATIONS.keep(seps)
}

/// Drop separations reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_separations(_ctx: Handle, seps: Handle) {
    SEPARATIONS.remove(seps);
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Check if any separations are active (spot behavior)
#[unsafe(no_mangle)]
pub extern "C" fn fz_separations_have_spots(_ctx: Handle, seps: Handle) -> i32 {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            for sep in &guard.seps {
                if sep.behavior == SeparationBehavior::Spot {
                    return 1;
                }
            }
        }
    }
    0
}

/// Get number of active (non-disabled) separations
#[unsafe(no_mangle)]
pub extern "C" fn fz_count_active_separations(_ctx: Handle, seps: Handle) -> i32 {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(guard) = seps_arc.lock() {
            return guard
                .seps
                .iter()
                .filter(|s| s.behavior != SeparationBehavior::Disabled)
                .count() as i32;
        }
    }
    0
}

/// Set all separations to composite mode
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_all_separations_to_composite(_ctx: Handle, seps: Handle) {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(mut guard) = seps_arc.lock() {
            if !guard.controllable {
                return;
            }
            for sep in &mut guard.seps {
                sep.behavior = SeparationBehavior::Composite;
            }
        }
    }
}

/// Set all separations to spot mode
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_all_separations_to_spot(_ctx: Handle, seps: Handle) {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(mut guard) = seps_arc.lock() {
            if !guard.controllable {
                return;
            }
            for sep in &mut guard.seps {
                if !sep.is_none {
                    sep.behavior = SeparationBehavior::Spot;
                }
            }
        }
    }
}

/// Disable all separations
#[unsafe(no_mangle)]
pub extern "C" fn fz_disable_all_separations(_ctx: Handle, seps: Handle) {
    if let Some(seps_arc) = SEPARATIONS.get(seps) {
        if let Ok(mut guard) = seps_arc.lock() {
            if !guard.controllable {
                return;
            }
            for sep in &mut guard.seps {
                sep.behavior = SeparationBehavior::Disabled;
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_separations() {
        let seps = fz_new_separations(0, 1);
        assert!(seps > 0);
        assert_eq!(fz_separations_controllable(0, seps), 1);
        assert_eq!(fz_count_separations(0, seps), 0);
        fz_drop_separations(0, seps);
    }

    #[test]
    fn test_add_separation() {
        let seps = fz_new_separations(0, 1);

        let name = c"PANTONE 185 C";
        let idx = fz_add_separation(0, seps, name.as_ptr(), 0, 0.0, 1.0, 0.9, 0.0);
        assert_eq!(idx, 0);
        assert_eq!(fz_count_separations(0, seps), 1);

        // Check behavior
        assert_eq!(
            fz_separation_current_behavior(0, seps, 0),
            SeparationBehavior::Composite as i32
        );

        fz_drop_separations(0, seps);
    }

    #[test]
    fn test_special_separations() {
        let seps = fz_new_separations(0, 1);

        let all_idx = fz_add_separation_all(0, seps);
        let none_idx = fz_add_separation_none(0, seps);

        assert_eq!(fz_count_separations(0, seps), 2);
        assert_eq!(fz_separation_is_all(0, seps, all_idx), 1);
        assert_eq!(fz_separation_is_none(0, seps, none_idx), 1);

        fz_drop_separations(0, seps);
    }

    #[test]
    fn test_separation_behavior() {
        let seps = fz_new_separations(0, 1);

        let name = c"Spot1";
        fz_add_separation(0, seps, name.as_ptr(), 0, 1.0, 0.0, 0.0, 0.0);

        // Default is composite
        assert_eq!(
            fz_separation_current_behavior(0, seps, 0),
            SeparationBehavior::Composite as i32
        );

        // Change to spot
        fz_set_separation_behavior(0, seps, 0, SeparationBehavior::Spot as i32);
        assert_eq!(
            fz_separation_current_behavior(0, seps, 0),
            SeparationBehavior::Spot as i32
        );

        // Should have spots now
        assert_eq!(fz_separations_have_spots(0, seps), 1);

        fz_drop_separations(0, seps);
    }

    #[test]
    fn test_non_controllable() {
        let seps = fz_new_separations(0, 0); // Not controllable

        let name = c"Spot1";
        fz_add_separation(0, seps, name.as_ptr(), 0, 1.0, 0.0, 0.0, 0.0);

        // Try to change behavior - should be ignored
        fz_set_separation_behavior(0, seps, 0, SeparationBehavior::Spot as i32);

        // Should still be composite
        assert_eq!(
            fz_separation_current_behavior(0, seps, 0),
            SeparationBehavior::Composite as i32
        );

        fz_drop_separations(0, seps);
    }

    #[test]
    fn test_cmyk_equivalent() {
        let seps = fz_new_separations(0, 1);

        let name = c"Spot1";
        fz_add_separation(0, seps, name.as_ptr(), 0, 0.1, 0.2, 0.3, 0.4);

        let mut cmyk = [0.0f32; 4];
        fz_separation_equivalent(0, seps, 0, cmyk.as_mut_ptr());

        assert_eq!(cmyk, [0.1, 0.2, 0.3, 0.4]);

        fz_drop_separations(0, seps);
    }

    #[test]
    fn test_clone_separations() {
        let seps = fz_new_separations(0, 1);

        let name = c"Spot1";
        fz_add_separation(0, seps, name.as_ptr(), 0, 1.0, 0.0, 0.0, 0.0);

        let cloned = fz_clone_separations(0, seps);
        assert!(cloned > 0);
        assert_eq!(fz_count_separations(0, cloned), 1);
        assert_eq!(fz_separations_equal(0, seps, cloned), 1);

        fz_drop_separations(0, seps);
        fz_drop_separations(0, cloned);
    }

    #[test]
    fn test_bulk_behavior_changes() {
        let seps = fz_new_separations(0, 1);

        let name1 = c"Spot1";
        let name2 = c"Spot2";
        fz_add_separation(0, seps, name1.as_ptr(), 0, 1.0, 0.0, 0.0, 0.0);
        fz_add_separation(0, seps, name2.as_ptr(), 0, 0.0, 1.0, 0.0, 0.0);

        // Set all to spot
        fz_set_all_separations_to_spot(0, seps);
        assert_eq!(fz_separations_have_spots(0, seps), 1);
        assert_eq!(fz_count_active_separations(0, seps), 2);

        // Disable all
        fz_disable_all_separations(0, seps);
        assert_eq!(fz_count_active_separations(0, seps), 0);

        // Reset to composite
        fz_set_all_separations_to_composite(0, seps);
        assert_eq!(fz_count_active_separations(0, seps), 2);

        fz_drop_separations(0, seps);
    }
}
