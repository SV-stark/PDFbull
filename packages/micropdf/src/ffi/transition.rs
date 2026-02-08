//! FFI bindings for fz_transition (Page Transitions)
//!
//! This module provides page transition effects for PDF presentations.
//! Supports various transition types including Split, Blinds, Box, Wipe,
//! Dissolve, Glitter, Fly, Push, Cover, Uncover, and Fade.

use std::sync::LazyLock;

use crate::ffi::{Handle, HandleStore};

/// Global store for transitions
pub static TRANSITIONS: LazyLock<HandleStore<Transition>> = LazyLock::new(HandleStore::new);

/// Transition types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionType {
    /// No transition (immediate replacement)
    #[default]
    None = 0,
    /// Split effect - page splits in two
    Split = 1,
    /// Blinds effect - venetian blind strips
    Blinds = 2,
    /// Box effect - box grows/shrinks
    Box = 3,
    /// Wipe effect - new page wipes over old
    Wipe = 4,
    /// Dissolve effect - pixels randomly change
    Dissolve = 5,
    /// Glitter effect - diagonal dissolve
    Glitter = 6,
    /// Fly effect - page flies in/out
    Fly = 7,
    /// Push effect - new page pushes old out
    Push = 8,
    /// Cover effect - new page slides over old
    Cover = 9,
    /// Uncover effect - old page slides away revealing new
    Uncover = 10,
    /// Fade effect - crossfade between pages
    Fade = 11,
}

impl TransitionType {
    /// Get transition type from integer
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => TransitionType::None,
            1 => TransitionType::Split,
            2 => TransitionType::Blinds,
            3 => TransitionType::Box,
            4 => TransitionType::Wipe,
            5 => TransitionType::Dissolve,
            6 => TransitionType::Glitter,
            7 => TransitionType::Fly,
            8 => TransitionType::Push,
            9 => TransitionType::Cover,
            10 => TransitionType::Uncover,
            11 => TransitionType::Fade,
            _ => TransitionType::None,
        }
    }

    /// Get transition name
    pub fn name(&self) -> &'static str {
        match self {
            TransitionType::None => "None",
            TransitionType::Split => "Split",
            TransitionType::Blinds => "Blinds",
            TransitionType::Box => "Box",
            TransitionType::Wipe => "Wipe",
            TransitionType::Dissolve => "Dissolve",
            TransitionType::Glitter => "Glitter",
            TransitionType::Fly => "Fly",
            TransitionType::Push => "Push",
            TransitionType::Cover => "Cover",
            TransitionType::Uncover => "Uncover",
            TransitionType::Fade => "Fade",
        }
    }
}

/// Page transition structure
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct Transition {
    /// Transition type
    pub transition_type: TransitionType,
    /// Effect duration in seconds
    pub duration: f32,
    /// Vertical orientation (0 = horizontal, 1 = vertical)
    pub vertical: i32,
    /// Direction (0 = inward, 1 = outward) for Split/Box
    pub outwards: i32,
    /// Direction angle in degrees (for Wipe, Glitter, Fly, etc.)
    pub direction: i32,
    /// Internal state variable 0
    pub state0: i32,
    /// Internal state variable 1
    pub state1: i32,
}

impl Transition {
    /// Create a new transition
    pub fn new(transition_type: TransitionType, duration: f32) -> Self {
        Self {
            transition_type,
            duration,
            vertical: 0,
            outwards: 0,
            direction: 0,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a split transition
    pub fn split(duration: f32, vertical: bool, outwards: bool) -> Self {
        Self {
            transition_type: TransitionType::Split,
            duration,
            vertical: if vertical { 1 } else { 0 },
            outwards: if outwards { 1 } else { 0 },
            direction: 0,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a blinds transition
    pub fn blinds(duration: f32, vertical: bool) -> Self {
        Self {
            transition_type: TransitionType::Blinds,
            duration,
            vertical: if vertical { 1 } else { 0 },
            outwards: 0,
            direction: 0,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a box transition
    pub fn box_transition(duration: f32, outwards: bool) -> Self {
        Self {
            transition_type: TransitionType::Box,
            duration,
            vertical: 0,
            outwards: if outwards { 1 } else { 0 },
            direction: 0,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a wipe transition
    pub fn wipe(duration: f32, direction: i32) -> Self {
        Self {
            transition_type: TransitionType::Wipe,
            duration,
            vertical: 0,
            outwards: 0,
            direction,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a dissolve transition
    pub fn dissolve(duration: f32) -> Self {
        Self {
            transition_type: TransitionType::Dissolve,
            duration,
            vertical: 0,
            outwards: 0,
            direction: 0,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a glitter transition
    pub fn glitter(duration: f32, direction: i32) -> Self {
        Self {
            transition_type: TransitionType::Glitter,
            duration,
            vertical: 0,
            outwards: 0,
            direction,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a fly transition
    pub fn fly(duration: f32, direction: i32, outwards: bool) -> Self {
        Self {
            transition_type: TransitionType::Fly,
            duration,
            vertical: 0,
            outwards: if outwards { 1 } else { 0 },
            direction,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a push transition
    pub fn push(duration: f32, direction: i32) -> Self {
        Self {
            transition_type: TransitionType::Push,
            duration,
            vertical: 0,
            outwards: 0,
            direction,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a cover transition
    pub fn cover(duration: f32, direction: i32) -> Self {
        Self {
            transition_type: TransitionType::Cover,
            duration,
            vertical: 0,
            outwards: 0,
            direction,
            state0: 0,
            state1: 0,
        }
    }

    /// Create an uncover transition
    pub fn uncover(duration: f32, direction: i32) -> Self {
        Self {
            transition_type: TransitionType::Uncover,
            duration,
            vertical: 0,
            outwards: 0,
            direction,
            state0: 0,
            state1: 0,
        }
    }

    /// Create a fade transition
    pub fn fade(duration: f32) -> Self {
        Self {
            transition_type: TransitionType::Fade,
            duration,
            vertical: 0,
            outwards: 0,
            direction: 0,
            state0: 0,
            state1: 0,
        }
    }
}

// ============================================================================
// Transition Frame Generation
// ============================================================================

/// Generate a transition frame between two pixmaps
///
/// This function blends the old pixmap (opix) and new pixmap (npix) into the
/// target pixmap (tpix) based on the transition type and progress (time).
///
/// # Arguments
/// * `tpix` - Target pixmap to write the blended result
/// * `opix` - Old (source) pixmap
/// * `npix` - New (destination) pixmap
/// * `time` - Progress through transition (0-256)
/// * `trans` - Transition parameters
///
/// # Returns
/// * `true` if frame was generated successfully
pub fn generate_transition_frame(
    tpix: &mut [u8],
    opix: &[u8],
    npix: &[u8],
    width: i32,
    height: i32,
    n: i32,
    time: i32,
    trans: &Transition,
) -> bool {
    if tpix.len() != opix.len() || opix.len() != npix.len() {
        return false;
    }

    let t = time.clamp(0, 256) as f32 / 256.0;
    let components = n as usize;

    match trans.transition_type {
        TransitionType::None => {
            // Immediate replacement
            tpix.copy_from_slice(if time >= 128 { npix } else { opix });
        }

        TransitionType::Fade => {
            // Crossfade between old and new
            for i in 0..tpix.len() {
                let old_val = opix[i] as f32;
                let new_val = npix[i] as f32;
                tpix[i] = (old_val * (1.0 - t) + new_val * t) as u8;
            }
        }

        TransitionType::Dissolve => {
            // Random pixel replacement based on progress
            let threshold = (t * 255.0) as u8;
            let stride = (width as usize) * components;

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let idx = y * stride + x * components;
                    // Simple deterministic "random" based on position
                    let hash = ((x * 7919 + y * 6271) % 256) as u8;
                    let use_new = hash < threshold;

                    for c in 0..components {
                        tpix[idx + c] = if use_new {
                            npix[idx + c]
                        } else {
                            opix[idx + c]
                        };
                    }
                }
            }
        }

        TransitionType::Wipe => {
            // Wipe from one direction
            let direction = trans.direction;
            let stride = (width as usize) * components;

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let idx = y * stride + x * components;

                    // Calculate progress at this pixel based on direction
                    let pixel_t = match direction {
                        0 | 360 => x as f32 / width as f32, // Left to right
                        90 => (height as usize - 1 - y) as f32 / height as f32, // Bottom to top
                        180 => (width as usize - 1 - x) as f32 / width as f32, // Right to left
                        270 => y as f32 / height as f32,    // Top to bottom
                        _ => x as f32 / width as f32,       // Default L->R
                    };

                    let use_new = pixel_t < t;

                    for c in 0..components {
                        tpix[idx + c] = if use_new {
                            npix[idx + c]
                        } else {
                            opix[idx + c]
                        };
                    }
                }
            }
        }

        TransitionType::Split => {
            // Split from center or edges
            let stride = (width as usize) * components;
            let hw = width as f32 / 2.0;
            let hh = height as f32 / 2.0;

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let idx = y * stride + x * components;

                    let pixel_t = if trans.vertical != 0 {
                        // Vertical split
                        let dist = (y as f32 - hh).abs() / hh;
                        if trans.outwards != 0 {
                            dist
                        } else {
                            1.0 - dist
                        }
                    } else {
                        // Horizontal split
                        let dist = (x as f32 - hw).abs() / hw;
                        if trans.outwards != 0 {
                            dist
                        } else {
                            1.0 - dist
                        }
                    };

                    let use_new = pixel_t < t;

                    for c in 0..components {
                        tpix[idx + c] = if use_new {
                            npix[idx + c]
                        } else {
                            opix[idx + c]
                        };
                    }
                }
            }
        }

        TransitionType::Blinds => {
            // Venetian blinds effect
            let stride = (width as usize) * components;
            let num_blinds = 10;

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let idx = y * stride + x * components;

                    let blind_pos = if trans.vertical != 0 {
                        (x as f32 / width as f32 * num_blinds as f32) % 1.0
                    } else {
                        (y as f32 / height as f32 * num_blinds as f32) % 1.0
                    };

                    let use_new = blind_pos < t;

                    for c in 0..components {
                        tpix[idx + c] = if use_new {
                            npix[idx + c]
                        } else {
                            opix[idx + c]
                        };
                    }
                }
            }
        }

        TransitionType::Box => {
            // Box grows or shrinks
            let stride = (width as usize) * components;
            let hw = width as f32 / 2.0;
            let hh = height as f32 / 2.0;

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let idx = y * stride + x * components;

                    let dx = (x as f32 - hw).abs() / hw;
                    let dy = (y as f32 - hh).abs() / hh;
                    let dist = dx.max(dy);

                    let pixel_t = if trans.outwards != 0 {
                        1.0 - dist
                    } else {
                        dist
                    };

                    let use_new = pixel_t < t;

                    for c in 0..components {
                        tpix[idx + c] = if use_new {
                            npix[idx + c]
                        } else {
                            opix[idx + c]
                        };
                    }
                }
            }
        }

        TransitionType::Glitter => {
            // Diagonal dissolve with glitter effect
            let stride = (width as usize) * components;
            let direction = trans.direction;

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let idx = y * stride + x * components;

                    // Diagonal progress based on direction
                    let diag = match direction {
                        0 | 360 => (x + y) as f32 / (width + height) as f32,
                        90 => (x + (height as usize - y)) as f32 / (width + height) as f32,
                        180 => {
                            ((width as usize - x) + (height as usize - y)) as f32
                                / (width + height) as f32
                        }
                        270 => ((width as usize - x) + y) as f32 / (width + height) as f32,
                        _ => (x + y) as f32 / (width + height) as f32,
                    };

                    // Add some randomness for glitter
                    let hash = ((x * 7919 + y * 6271) % 64) as f32 / 256.0;
                    let pixel_t = diag + hash * 0.1;

                    let use_new = pixel_t < t;

                    for c in 0..components {
                        tpix[idx + c] = if use_new {
                            npix[idx + c]
                        } else {
                            opix[idx + c]
                        };
                    }
                }
            }
        }

        TransitionType::Push | TransitionType::Cover | TransitionType::Uncover => {
            // Sliding transitions
            let stride = (width as usize) * components;
            let direction = trans.direction;

            let (dx, dy) = match direction {
                0 | 360 => ((t * width as f32) as i32, 0), // Push from right
                90 => (0, (t * height as f32) as i32),     // Push from top
                180 => (-((t * width as f32) as i32), 0),  // Push from left
                270 => (0, -((t * height as f32) as i32)), // Push from bottom
                _ => ((t * width as f32) as i32, 0),
            };

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let idx = y * stride + x * components;

                    // Calculate source coordinates
                    let (src_new, src_old) = match trans.transition_type {
                        TransitionType::Push => {
                            let nx = (x as i32 - dx).clamp(0, width - 1) as usize;
                            let ny = (y as i32 - dy).clamp(0, height - 1) as usize;
                            let ox = (x as i32 + (width - dx.abs())).clamp(0, width - 1) as usize;
                            let oy = (y as i32 + (height - dy.abs())).clamp(0, height - 1) as usize;
                            ((nx, ny), (ox, oy))
                        }
                        TransitionType::Cover => {
                            let nx = (x as i32 - dx).clamp(0, width - 1) as usize;
                            let ny = (y as i32 - dy).clamp(0, height - 1) as usize;
                            ((nx, ny), (x, y))
                        }
                        TransitionType::Uncover => {
                            let ox = (x as i32 + dx).clamp(0, width - 1) as usize;
                            let oy = (y as i32 + dy).clamp(0, height - 1) as usize;
                            ((x, y), (ox, oy))
                        }
                        _ => ((x, y), (x, y)),
                    };

                    // Determine if this pixel shows new or old content
                    let show_new = match direction {
                        0 | 360 => (x as i32) < dx,
                        90 => (y as i32) < dy,
                        180 => (x as i32) >= (width + dx),
                        270 => (y as i32) >= (height + dy),
                        _ => (x as i32) < dx,
                    };

                    let src_idx = if show_new {
                        src_new.1 * stride + src_new.0 * components
                    } else {
                        src_old.1 * stride + src_old.0 * components
                    };

                    for c in 0..components {
                        tpix[idx + c] = if show_new {
                            npix.get(src_idx + c).copied().unwrap_or(0)
                        } else {
                            opix.get(src_idx + c).copied().unwrap_or(0)
                        };
                    }
                }
            }
        }

        TransitionType::Fly => {
            // Fly in/out effect (combines scaling with movement)
            let stride = (width as usize) * components;

            // For fly, we scale while moving
            let scale = if trans.outwards != 0 {
                1.0 - t * 0.5 // Fly out: shrink
            } else {
                0.5 + t * 0.5 // Fly in: grow
            };

            let center_x = width as f32 / 2.0;
            let center_y = height as f32 / 2.0;

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let idx = y * stride + x * components;

                    // Calculate source position with scaling from center
                    let sx = center_x + (x as f32 - center_x) / scale;
                    let sy = center_y + (y as f32 - center_y) / scale;

                    let in_bounds =
                        sx >= 0.0 && sx < width as f32 && sy >= 0.0 && sy < height as f32;

                    if trans.outwards != 0 {
                        // Fly out: old content scales down
                        if in_bounds {
                            let src_idx = (sy as usize) * stride + (sx as usize) * components;
                            let blend = (1.0 - t).clamp(0.0, 1.0);
                            for c in 0..components {
                                let old_val = opix.get(src_idx + c).copied().unwrap_or(0) as f32;
                                let new_val = npix[idx + c] as f32;
                                tpix[idx + c] = (old_val * blend + new_val * (1.0 - blend)) as u8;
                            }
                        } else {
                            for c in 0..components {
                                tpix[idx + c] = npix[idx + c];
                            }
                        }
                    } else {
                        // Fly in: new content scales up
                        if in_bounds && t > 0.1 {
                            let src_idx = (sy as usize) * stride + (sx as usize) * components;
                            let blend = t.clamp(0.0, 1.0);
                            for c in 0..components {
                                let old_val = opix[idx + c] as f32;
                                let new_val = npix.get(src_idx + c).copied().unwrap_or(0) as f32;
                                tpix[idx + c] = (old_val * (1.0 - blend) + new_val * blend) as u8;
                            }
                        } else {
                            for c in 0..components {
                                tpix[idx + c] = opix[idx + c];
                            }
                        }
                    }
                }
            }
        }
    }

    true
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_transition(transition_type: i32, duration: f32) -> Handle {
    let trans = Transition::new(TransitionType::from_i32(transition_type), duration);
    TRANSITIONS.insert(trans)
}

/// Create a split transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_split_transition(duration: f32, vertical: i32, outwards: i32) -> Handle {
    let trans = Transition::split(duration, vertical != 0, outwards != 0);
    TRANSITIONS.insert(trans)
}

/// Create a blinds transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_blinds_transition(duration: f32, vertical: i32) -> Handle {
    let trans = Transition::blinds(duration, vertical != 0);
    TRANSITIONS.insert(trans)
}

/// Create a box transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_box_transition(duration: f32, outwards: i32) -> Handle {
    let trans = Transition::box_transition(duration, outwards != 0);
    TRANSITIONS.insert(trans)
}

/// Create a wipe transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_wipe_transition(duration: f32, direction: i32) -> Handle {
    let trans = Transition::wipe(duration, direction);
    TRANSITIONS.insert(trans)
}

/// Create a dissolve transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_dissolve_transition(duration: f32) -> Handle {
    let trans = Transition::dissolve(duration);
    TRANSITIONS.insert(trans)
}

/// Create a glitter transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_glitter_transition(duration: f32, direction: i32) -> Handle {
    let trans = Transition::glitter(duration, direction);
    TRANSITIONS.insert(trans)
}

/// Create a fly transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_fly_transition(duration: f32, direction: i32, outwards: i32) -> Handle {
    let trans = Transition::fly(duration, direction, outwards != 0);
    TRANSITIONS.insert(trans)
}

/// Create a push transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_push_transition(duration: f32, direction: i32) -> Handle {
    let trans = Transition::push(duration, direction);
    TRANSITIONS.insert(trans)
}

/// Create a cover transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_cover_transition(duration: f32, direction: i32) -> Handle {
    let trans = Transition::cover(duration, direction);
    TRANSITIONS.insert(trans)
}

/// Create an uncover transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_uncover_transition(duration: f32, direction: i32) -> Handle {
    let trans = Transition::uncover(duration, direction);
    TRANSITIONS.insert(trans)
}

/// Create a fade transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_fade_transition(duration: f32) -> Handle {
    let trans = Transition::fade(duration);
    TRANSITIONS.insert(trans)
}

/// Drop a transition
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_transition(_ctx: Handle, trans: Handle) {
    TRANSITIONS.remove(trans);
}

/// Generate a transition frame between two pixmaps
///
/// # Arguments
/// * `ctx` - Context handle
/// * `tpix` - Target pixmap handle
/// * `opix` - Old pixmap handle
/// * `npix` - New pixmap handle
/// * `time` - Progress (0-256)
/// * `trans` - Transition handle
///
/// # Returns
/// * 1 on success, 0 on failure
#[unsafe(no_mangle)]
pub extern "C" fn fz_generate_transition(
    _ctx: Handle,
    tpix: Handle,
    opix: Handle,
    npix: Handle,
    time: i32,
    trans: Handle,
) -> i32 {
    // Get transition
    let transition = match TRANSITIONS.get(trans) {
        Some(t) => t.lock().unwrap().clone(),
        None => return 0,
    };

    // Get pixmaps
    let tpix_arc = match crate::ffi::PIXMAPS.get(tpix) {
        Some(p) => p,
        None => return 0,
    };

    let opix_arc = match crate::ffi::PIXMAPS.get(opix) {
        Some(p) => p,
        None => return 0,
    };

    let npix_arc = match crate::ffi::PIXMAPS.get(npix) {
        Some(p) => p,
        None => return 0,
    };

    // Get pixmap data
    let mut tpix_guard = tpix_arc.lock().unwrap();
    let opix_guard = opix_arc.lock().unwrap();
    let npix_guard = npix_arc.lock().unwrap();

    // Verify dimensions match
    if tpix_guard.w() != opix_guard.w()
        || tpix_guard.w() != npix_guard.w()
        || tpix_guard.h() != opix_guard.h()
        || tpix_guard.h() != npix_guard.h()
        || tpix_guard.n() != opix_guard.n()
        || tpix_guard.n() != npix_guard.n()
    {
        return 0;
    }

    let width = tpix_guard.w();
    let height = tpix_guard.h();
    let n = tpix_guard.n();

    // Get mutable samples from target pixmap
    let tpix_samples = tpix_guard.samples_mut();
    let opix_samples = opix_guard.samples();
    let npix_samples = npix_guard.samples();

    // Generate the transition frame
    if generate_transition_frame(
        tpix_samples,
        opix_samples,
        npix_samples,
        width,
        height,
        n,
        time,
        &transition,
    ) {
        1
    } else {
        0
    }
}

// ============================================================================
// Transition Property Accessors
// ============================================================================

/// Get transition type
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_type(trans: Handle) -> i32 {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().transition_type as i32
    } else {
        0
    }
}

/// Get transition duration
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_duration(trans: Handle) -> f32 {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().duration
    } else {
        0.0
    }
}

/// Set transition duration
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_set_duration(trans: Handle, duration: f32) {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().duration = duration;
    }
}

/// Get transition vertical flag
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_vertical(trans: Handle) -> i32 {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().vertical
    } else {
        0
    }
}

/// Set transition vertical flag
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_set_vertical(trans: Handle, vertical: i32) {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().vertical = vertical;
    }
}

/// Get transition outwards flag
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_outwards(trans: Handle) -> i32 {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().outwards
    } else {
        0
    }
}

/// Set transition outwards flag
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_set_outwards(trans: Handle, outwards: i32) {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().outwards = outwards;
    }
}

/// Get transition direction
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_direction(trans: Handle) -> i32 {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().direction
    } else {
        0
    }
}

/// Set transition direction
#[unsafe(no_mangle)]
pub extern "C" fn fz_transition_set_direction(trans: Handle, direction: i32) {
    if let Some(t) = TRANSITIONS.get(trans) {
        t.lock().unwrap().direction = direction;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_type_from_i32() {
        assert_eq!(TransitionType::from_i32(0), TransitionType::None);
        assert_eq!(TransitionType::from_i32(1), TransitionType::Split);
        assert_eq!(TransitionType::from_i32(5), TransitionType::Dissolve);
        assert_eq!(TransitionType::from_i32(11), TransitionType::Fade);
        assert_eq!(TransitionType::from_i32(99), TransitionType::None);
    }

    #[test]
    fn test_transition_type_name() {
        assert_eq!(TransitionType::None.name(), "None");
        assert_eq!(TransitionType::Fade.name(), "Fade");
        assert_eq!(TransitionType::Dissolve.name(), "Dissolve");
    }

    #[test]
    fn test_new_transition() {
        let trans = fz_new_transition(TransitionType::Fade as i32, 1.5);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Fade as i32);
        assert!((fz_transition_duration(trans) - 1.5).abs() < 0.01);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_split_transition() {
        let trans = fz_new_split_transition(2.0, 1, 0);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Split as i32);
        assert_eq!(fz_transition_vertical(trans), 1);
        assert_eq!(fz_transition_outwards(trans), 0);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_blinds_transition() {
        let trans = fz_new_blinds_transition(1.0, 0);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Blinds as i32);
        assert_eq!(fz_transition_vertical(trans), 0);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_box_transition() {
        let trans = fz_new_box_transition(1.5, 1);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Box as i32);
        assert_eq!(fz_transition_outwards(trans), 1);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_wipe_transition() {
        let trans = fz_new_wipe_transition(1.0, 90);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Wipe as i32);
        assert_eq!(fz_transition_direction(trans), 90);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_dissolve_transition() {
        let trans = fz_new_dissolve_transition(2.0);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Dissolve as i32);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_glitter_transition() {
        let trans = fz_new_glitter_transition(1.5, 45);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Glitter as i32);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_fly_transition() {
        let trans = fz_new_fly_transition(1.0, 0, 1);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Fly as i32);
        assert_eq!(fz_transition_outwards(trans), 1);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_push_transition() {
        let trans = fz_new_push_transition(0.5, 180);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Push as i32);
        assert_eq!(fz_transition_direction(trans), 180);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_cover_transition() {
        let trans = fz_new_cover_transition(0.75, 270);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Cover as i32);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_uncover_transition() {
        let trans = fz_new_uncover_transition(0.75, 90);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Uncover as i32);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_fade_transition() {
        let trans = fz_new_fade_transition(1.0);
        assert!(trans > 0);

        assert_eq!(fz_transition_type(trans), TransitionType::Fade as i32);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_set_duration() {
        let trans = fz_new_fade_transition(1.0);

        fz_transition_set_duration(trans, 2.5);
        assert!((fz_transition_duration(trans) - 2.5).abs() < 0.01);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_set_direction() {
        let trans = fz_new_wipe_transition(1.0, 0);

        fz_transition_set_direction(trans, 180);
        assert_eq!(fz_transition_direction(trans), 180);

        fz_drop_transition(0, trans);
    }

    #[test]
    fn test_all_transition_types() {
        let transitions = vec![
            fz_new_transition(0, 1.0),  // None
            fz_new_transition(1, 1.0),  // Split
            fz_new_transition(2, 1.0),  // Blinds
            fz_new_transition(3, 1.0),  // Box
            fz_new_transition(4, 1.0),  // Wipe
            fz_new_transition(5, 1.0),  // Dissolve
            fz_new_transition(6, 1.0),  // Glitter
            fz_new_transition(7, 1.0),  // Fly
            fz_new_transition(8, 1.0),  // Push
            fz_new_transition(9, 1.0),  // Cover
            fz_new_transition(10, 1.0), // Uncover
            fz_new_transition(11, 1.0), // Fade
        ];

        for (i, trans) in transitions.iter().enumerate() {
            assert!(*trans > 0);
            assert_eq!(fz_transition_type(*trans), i as i32);
            fz_drop_transition(0, *trans);
        }
    }

    #[test]
    fn test_generate_transition_frame_fade() {
        // Test fade transition with raw buffers
        let width = 4;
        let height = 4;
        let n = 4; // RGBA

        let mut tpix = vec![0u8; (width * height * n) as usize];
        let opix = vec![255u8; (width * height * n) as usize]; // White
        let npix = vec![0u8; (width * height * n) as usize]; // Black

        let trans = Transition::fade(1.0);

        // At time 128 (50%), should be gray
        let result =
            generate_transition_frame(&mut tpix, &opix, &npix, width, height, n, 128, &trans);
        assert!(result);

        // Check that pixels are approximately 50% blended
        let expected = 127; // 255 * 0.5
        assert!((tpix[0] as i32 - expected as i32).abs() < 5);
    }

    #[test]
    fn test_generate_transition_frame_none() {
        let width = 2;
        let height = 2;
        let n = 4;

        let mut tpix = vec![0u8; (width * height * n) as usize];
        let opix = vec![100u8; (width * height * n) as usize];
        let npix = vec![200u8; (width * height * n) as usize];

        let trans = Transition::new(TransitionType::None, 0.0);

        // At time < 128, should show old
        let result =
            generate_transition_frame(&mut tpix, &opix, &npix, width, height, n, 64, &trans);
        assert!(result);
        assert_eq!(tpix[0], 100);

        // At time >= 128, should show new
        let result =
            generate_transition_frame(&mut tpix, &opix, &npix, width, height, n, 192, &trans);
        assert!(result);
        assert_eq!(tpix[0], 200);
    }
}
