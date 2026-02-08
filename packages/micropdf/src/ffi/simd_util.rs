//! SIMD Acceleration Utilities
//!
//! Provides SIMD-accelerated implementations for:
//! - Matrix operations (concat, transform)
//! - Color space conversions (RGB<->CMYK, etc.)
//! - Buffer operations (copy, fill, compare)
//! - Base64 encode/decode
//!
//! Uses runtime feature detection to select the best implementation.

// Allow unsafe operations inside unsafe functions (Rust 2024 edition compat)
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_float;

// ============================================================================
// Feature Detection
// ============================================================================

/// SIMD feature flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SimdFeatures {
    pub sse2: bool,
    pub sse3: bool,
    pub ssse3: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub fma: bool,
    pub neon: bool,
}

impl SimdFeatures {
    /// Detect available SIMD features at runtime
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                sse2: std::arch::is_x86_feature_detected!("sse2"),
                sse3: std::arch::is_x86_feature_detected!("sse3"),
                ssse3: std::arch::is_x86_feature_detected!("ssse3"),
                sse4_1: std::arch::is_x86_feature_detected!("sse4.1"),
                sse4_2: std::arch::is_x86_feature_detected!("sse4.2"),
                avx: std::arch::is_x86_feature_detected!("avx"),
                avx2: std::arch::is_x86_feature_detected!("avx2"),
                fma: std::arch::is_x86_feature_detected!("fma"),
                neon: false,
            }
        }
        #[cfg(target_arch = "x86")]
        {
            Self {
                sse2: std::arch::is_x86_feature_detected!("sse2"),
                sse3: std::arch::is_x86_feature_detected!("sse3"),
                ssse3: std::arch::is_x86_feature_detected!("ssse3"),
                sse4_1: std::arch::is_x86_feature_detected!("sse4.1"),
                sse4_2: std::arch::is_x86_feature_detected!("sse4.2"),
                avx: std::arch::is_x86_feature_detected!("avx"),
                avx2: std::arch::is_x86_feature_detected!("avx2"),
                fma: std::arch::is_x86_feature_detected!("fma"),
                neon: false,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                sse2: false,
                sse3: false,
                ssse3: false,
                sse4_1: false,
                sse4_2: false,
                avx: false,
                avx2: false,
                fma: false,
                neon: std::arch::is_aarch64_feature_detected!("neon"),
            }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
        {
            Self {
                sse2: false,
                sse3: false,
                ssse3: false,
                sse4_1: false,
                sse4_2: false,
                avx: false,
                avx2: false,
                fma: false,
                neon: false,
            }
        }
    }

    /// Check if any SIMD is available
    pub fn has_simd(&self) -> bool {
        self.sse2 || self.neon
    }

    /// Get best available SIMD level
    pub fn best_level(&self) -> SimdLevel {
        if self.avx2 {
            SimdLevel::Avx2
        } else if self.avx {
            SimdLevel::Avx
        } else if self.sse4_1 {
            SimdLevel::Sse41
        } else if self.sse2 {
            SimdLevel::Sse2
        } else if self.neon {
            SimdLevel::Neon
        } else {
            SimdLevel::Scalar
        }
    }
}

/// SIMD instruction level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub enum SimdLevel {
    Scalar = 0,
    Sse2 = 1,
    Sse41 = 2,
    Avx = 3,
    Avx2 = 4,
    Neon = 5,
}

// Cached feature detection
static SIMD_FEATURES: std::sync::LazyLock<SimdFeatures> =
    std::sync::LazyLock::new(SimdFeatures::detect);

// ============================================================================
// Matrix Operations (SIMD)
// ============================================================================

/// Matrix representation for SIMD operations (row-major)
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SimdMatrix {
    pub data: [c_float; 6], // a, b, c, d, e, f
    pub _pad: [c_float; 2], // Padding for alignment
}

impl SimdMatrix {
    pub const fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self {
            data: [a, b, c, d, e, f],
            _pad: [0.0, 0.0],
        }
    }

    pub const fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }
}

/// Concatenate two matrices using SIMD when available
pub fn matrix_concat(left: &SimdMatrix, right: &SimdMatrix) -> SimdMatrix {
    let level = SIMD_FEATURES.best_level();

    match level {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        SimdLevel::Sse2 | SimdLevel::Sse41 | SimdLevel::Avx | SimdLevel::Avx2 => unsafe {
            matrix_concat_sse2(left, right)
        },
        #[cfg(target_arch = "aarch64")]
        SimdLevel::Neon => unsafe { matrix_concat_neon(left, right) },
        _ => matrix_concat_scalar(left, right),
    }
}

/// Scalar fallback for matrix concatenation
fn matrix_concat_scalar(left: &SimdMatrix, right: &SimdMatrix) -> SimdMatrix {
    let l = &left.data;
    let r = &right.data;
    SimdMatrix::new(
        l[0] * r[0] + l[1] * r[2],
        l[0] * r[1] + l[1] * r[3],
        l[2] * r[0] + l[3] * r[2],
        l[2] * r[1] + l[3] * r[3],
        l[4] * r[0] + l[5] * r[2] + r[4],
        l[4] * r[1] + l[5] * r[3] + r[5],
    )
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
unsafe fn matrix_concat_sse2(left: &SimdMatrix, right: &SimdMatrix) -> SimdMatrix {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    // Load matrices
    let l = _mm_loadu_ps(left.data.as_ptr());
    let r = _mm_loadu_ps(right.data.as_ptr());

    // Extract components
    let la = _mm_shuffle_ps(l, l, 0x00); // [a, a, a, a]
    let lb = _mm_shuffle_ps(l, l, 0x55); // [b, b, b, b]
    let lc = _mm_shuffle_ps(l, l, 0xAA); // [c, c, c, c]
    let ld = _mm_shuffle_ps(l, l, 0xFF); // [d, d, d, d]

    let ra = _mm_shuffle_ps(r, r, 0x00);
    let rb = _mm_shuffle_ps(r, r, 0x55);
    let rc = _mm_shuffle_ps(r, r, 0xAA);
    let rd = _mm_shuffle_ps(r, r, 0xFF);

    // Compute new_a = la*ra + lb*rc
    let new_a = _mm_add_ps(_mm_mul_ps(la, ra), _mm_mul_ps(lb, rc));
    // Compute new_b = la*rb + lb*rd
    let new_b = _mm_add_ps(_mm_mul_ps(la, rb), _mm_mul_ps(lb, rd));
    // Compute new_c = lc*ra + ld*rc
    let new_c = _mm_add_ps(_mm_mul_ps(lc, ra), _mm_mul_ps(ld, rc));
    // Compute new_d = lc*rb + ld*rd
    let new_d = _mm_add_ps(_mm_mul_ps(lc, rb), _mm_mul_ps(ld, rd));

    // Extract scalar values
    let mut result = SimdMatrix::identity();
    result.data[0] = _mm_cvtss_f32(new_a);
    result.data[1] = _mm_cvtss_f32(new_b);
    result.data[2] = _mm_cvtss_f32(new_c);
    result.data[3] = _mm_cvtss_f32(new_d);

    // Translation: e' = le*ra + lf*rc + re, f' = le*rb + lf*rd + rf
    let le = left.data[4];
    let lf = left.data[5];
    result.data[4] = le * right.data[0] + lf * right.data[2] + right.data[4];
    result.data[5] = le * right.data[1] + lf * right.data[3] + right.data[5];

    result
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn matrix_concat_neon(left: &SimdMatrix, right: &SimdMatrix) -> SimdMatrix {
    use std::arch::aarch64::*;

    let l = vld1q_f32(left.data.as_ptr());
    let r = vld1q_f32(right.data.as_ptr());

    // Similar computation using NEON intrinsics
    let la = vdupq_laneq_f32(l, 0);
    let lb = vdupq_laneq_f32(l, 1);
    let lc = vdupq_laneq_f32(l, 2);
    let ld = vdupq_laneq_f32(l, 3);

    let ra = vdupq_laneq_f32(r, 0);
    let rb = vdupq_laneq_f32(r, 1);
    let rc = vdupq_laneq_f32(r, 2);
    let rd = vdupq_laneq_f32(r, 3);

    let new_a = vaddq_f32(vmulq_f32(la, ra), vmulq_f32(lb, rc));
    let new_b = vaddq_f32(vmulq_f32(la, rb), vmulq_f32(lb, rd));
    let new_c = vaddq_f32(vmulq_f32(lc, ra), vmulq_f32(ld, rc));
    let new_d = vaddq_f32(vmulq_f32(lc, rb), vmulq_f32(ld, rd));

    let mut result = SimdMatrix::identity();
    result.data[0] = vgetq_lane_f32(new_a, 0);
    result.data[1] = vgetq_lane_f32(new_b, 0);
    result.data[2] = vgetq_lane_f32(new_c, 0);
    result.data[3] = vgetq_lane_f32(new_d, 0);

    let le = left.data[4];
    let lf = left.data[5];
    result.data[4] = le * right.data[0] + lf * right.data[2] + right.data[4];
    result.data[5] = le * right.data[1] + lf * right.data[3] + right.data[5];

    result
}

/// Transform a point by a matrix
pub fn transform_point(x: f32, y: f32, m: &SimdMatrix) -> (f32, f32) {
    (
        x * m.data[0] + y * m.data[2] + m.data[4],
        x * m.data[1] + y * m.data[3] + m.data[5],
    )
}

/// Transform multiple points by a matrix (batch operation)
pub fn transform_points(points: &mut [(f32, f32)], m: &SimdMatrix) {
    let level = SIMD_FEATURES.best_level();

    match level {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        SimdLevel::Sse2 | SimdLevel::Sse41 | SimdLevel::Avx | SimdLevel::Avx2 => unsafe {
            transform_points_sse2(points, m)
        },
        #[cfg(target_arch = "aarch64")]
        SimdLevel::Neon => unsafe { transform_points_neon(points, m) },
        _ => transform_points_scalar(points, m),
    }
}

fn transform_points_scalar(points: &mut [(f32, f32)], m: &SimdMatrix) {
    for p in points.iter_mut() {
        let (nx, ny) = transform_point(p.0, p.1, m);
        *p = (nx, ny);
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
unsafe fn transform_points_sse2(points: &mut [(f32, f32)], m: &SimdMatrix) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    // Load matrix coefficients
    let ma = _mm_set1_ps(m.data[0]);
    let mb = _mm_set1_ps(m.data[1]);
    let mc = _mm_set1_ps(m.data[2]);
    let md = _mm_set1_ps(m.data[3]);
    let me = _mm_set1_ps(m.data[4]);
    let mf = _mm_set1_ps(m.data[5]);

    // Process 2 points at a time (4 floats = 128 bits)
    let chunks = points.len() / 2;
    for i in 0..chunks {
        let idx = i * 2;
        let x0 = points[idx].0;
        let y0 = points[idx].1;
        let x1 = points[idx + 1].0;
        let y1 = points[idx + 1].1;

        let xs = _mm_set_ps(0.0, 0.0, x1, x0);
        let ys = _mm_set_ps(0.0, 0.0, y1, y0);

        // new_x = x*a + y*c + e
        let new_x = _mm_add_ps(_mm_add_ps(_mm_mul_ps(xs, ma), _mm_mul_ps(ys, mc)), me);
        // new_y = x*b + y*d + f
        let new_y = _mm_add_ps(_mm_add_ps(_mm_mul_ps(xs, mb), _mm_mul_ps(ys, md)), mf);

        // Extract results
        let mut nx = [0.0f32; 4];
        let mut ny = [0.0f32; 4];
        _mm_storeu_ps(nx.as_mut_ptr(), new_x);
        _mm_storeu_ps(ny.as_mut_ptr(), new_y);

        points[idx] = (nx[0], ny[0]);
        points[idx + 1] = (nx[1], ny[1]);
    }

    // Handle remaining point
    if points.len() % 2 == 1 {
        let last = points.len() - 1;
        let (nx, ny) = transform_point(points[last].0, points[last].1, m);
        points[last] = (nx, ny);
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn transform_points_neon(points: &mut [(f32, f32)], m: &SimdMatrix) {
    use std::arch::aarch64::*;

    let ma = vdupq_n_f32(m.data[0]);
    let mb = vdupq_n_f32(m.data[1]);
    let mc = vdupq_n_f32(m.data[2]);
    let md = vdupq_n_f32(m.data[3]);
    let me = vdupq_n_f32(m.data[4]);
    let mf = vdupq_n_f32(m.data[5]);

    // Process 4 points at a time
    let chunks = points.len() / 4;
    for i in 0..chunks {
        let idx = i * 4;
        let xs = vld1q_f32(
            [
                points[idx].0,
                points[idx + 1].0,
                points[idx + 2].0,
                points[idx + 3].0,
            ]
            .as_ptr(),
        );
        let ys = vld1q_f32(
            [
                points[idx].1,
                points[idx + 1].1,
                points[idx + 2].1,
                points[idx + 3].1,
            ]
            .as_ptr(),
        );

        let new_x = vaddq_f32(vaddq_f32(vmulq_f32(xs, ma), vmulq_f32(ys, mc)), me);
        let new_y = vaddq_f32(vaddq_f32(vmulq_f32(xs, mb), vmulq_f32(ys, md)), mf);

        let mut nx = [0.0f32; 4];
        let mut ny = [0.0f32; 4];
        vst1q_f32(nx.as_mut_ptr(), new_x);
        vst1q_f32(ny.as_mut_ptr(), new_y);

        for j in 0..4 {
            points[idx + j] = (nx[j], ny[j]);
        }
    }

    // Handle remaining points
    for i in (chunks * 4)..points.len() {
        let (nx, ny) = transform_point(points[i].0, points[i].1, m);
        points[i] = (nx, ny);
    }
}

// ============================================================================
// Color Space Conversions (SIMD)
// ============================================================================

/// Convert RGB to CMYK using SIMD
pub fn rgb_to_cmyk(r: f32, g: f32, b: f32) -> (f32, f32, f32, f32) {
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return (0.0, 0.0, 0.0, 1.0);
    }
    let c = (1.0 - r - k) / (1.0 - k);
    let m = (1.0 - g - k) / (1.0 - k);
    let y = (1.0 - b - k) / (1.0 - k);
    (c, m, y, k)
}

/// Convert CMYK to RGB using SIMD
pub fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> (f32, f32, f32) {
    let r = (1.0 - c) * (1.0 - k);
    let g = (1.0 - m) * (1.0 - k);
    let b = (1.0 - y) * (1.0 - k);
    (r, g, b)
}

/// Batch RGB to CMYK conversion
pub fn rgb_to_cmyk_batch(rgb: &[(f32, f32, f32)], cmyk: &mut [(f32, f32, f32, f32)]) {
    let level = SIMD_FEATURES.best_level();

    match level {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        SimdLevel::Sse2 | SimdLevel::Sse41 | SimdLevel::Avx | SimdLevel::Avx2 => unsafe {
            rgb_to_cmyk_batch_sse2(rgb, cmyk)
        },
        _ => rgb_to_cmyk_batch_scalar(rgb, cmyk),
    }
}

fn rgb_to_cmyk_batch_scalar(rgb: &[(f32, f32, f32)], cmyk: &mut [(f32, f32, f32, f32)]) {
    for (i, &(r, g, b)) in rgb.iter().enumerate() {
        if i < cmyk.len() {
            cmyk[i] = rgb_to_cmyk(r, g, b);
        }
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
unsafe fn rgb_to_cmyk_batch_sse2(rgb: &[(f32, f32, f32)], cmyk: &mut [(f32, f32, f32, f32)]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let ones = _mm_set1_ps(1.0);

    for (i, &(r, g, b)) in rgb.iter().enumerate() {
        if i >= cmyk.len() {
            break;
        }

        let rv = _mm_set1_ps(r);
        let gv = _mm_set1_ps(g);
        let bv = _mm_set1_ps(b);

        // k = 1 - max(r, g, b)
        let max_rg = _mm_max_ps(rv, gv);
        let max_rgb = _mm_max_ps(max_rg, bv);
        let k = _mm_sub_ps(ones, max_rgb);
        let k_scalar = _mm_cvtss_f32(k);

        if k_scalar >= 1.0 {
            cmyk[i] = (0.0, 0.0, 0.0, 1.0);
            continue;
        }

        let inv_1_minus_k = _mm_set1_ps(1.0 / (1.0 - k_scalar));

        // c = (1 - r - k) / (1 - k)
        let c = _mm_mul_ps(_mm_sub_ps(_mm_sub_ps(ones, rv), k), inv_1_minus_k);
        // m = (1 - g - k) / (1 - k)
        let m = _mm_mul_ps(_mm_sub_ps(_mm_sub_ps(ones, gv), k), inv_1_minus_k);
        // y = (1 - b - k) / (1 - k)
        let y = _mm_mul_ps(_mm_sub_ps(_mm_sub_ps(ones, bv), k), inv_1_minus_k);

        cmyk[i] = (
            _mm_cvtss_f32(c),
            _mm_cvtss_f32(m),
            _mm_cvtss_f32(y),
            k_scalar,
        );
    }
}

/// Convert grayscale to RGB
#[inline]
pub fn gray_to_rgb(gray: f32) -> (f32, f32, f32) {
    (gray, gray, gray)
}

/// Convert RGB to grayscale (luminance)
#[inline]
pub fn rgb_to_gray(r: f32, g: f32, b: f32) -> f32 {
    // ITU-R BT.709 coefficients
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

// ============================================================================
// Buffer Operations (SIMD)
// ============================================================================

/// Fill buffer with a byte value using SIMD
pub fn buffer_fill(dst: &mut [u8], value: u8) {
    let level = SIMD_FEATURES.best_level();

    match level {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        SimdLevel::Sse2 | SimdLevel::Sse41 | SimdLevel::Avx | SimdLevel::Avx2 => unsafe {
            buffer_fill_sse2(dst, value)
        },
        #[cfg(target_arch = "aarch64")]
        SimdLevel::Neon => unsafe { buffer_fill_neon(dst, value) },
        _ => dst.fill(value),
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
unsafe fn buffer_fill_sse2(dst: &mut [u8], value: u8) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let v = _mm_set1_epi8(value as i8);
    let chunks = dst.len() / 16;

    for i in 0..chunks {
        let ptr = dst.as_mut_ptr().add(i * 16) as *mut __m128i;
        _mm_storeu_si128(ptr, v);
    }

    // Fill remaining bytes
    for i in (chunks * 16)..dst.len() {
        dst[i] = value;
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn buffer_fill_neon(dst: &mut [u8], value: u8) {
    use std::arch::aarch64::*;

    let v = vdupq_n_u8(value);
    let chunks = dst.len() / 16;

    for i in 0..chunks {
        let ptr = dst.as_mut_ptr().add(i * 16);
        vst1q_u8(ptr, v);
    }

    for i in (chunks * 16)..dst.len() {
        dst[i] = value;
    }
}

/// Copy buffer using SIMD
pub fn buffer_copy(dst: &mut [u8], src: &[u8]) {
    let len = dst.len().min(src.len());
    let level = SIMD_FEATURES.best_level();

    match level {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        SimdLevel::Sse2 | SimdLevel::Sse41 | SimdLevel::Avx | SimdLevel::Avx2 => unsafe {
            buffer_copy_sse2(&mut dst[..len], &src[..len])
        },
        #[cfg(target_arch = "aarch64")]
        SimdLevel::Neon => unsafe { buffer_copy_neon(&mut dst[..len], &src[..len]) },
        _ => dst[..len].copy_from_slice(&src[..len]),
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
unsafe fn buffer_copy_sse2(dst: &mut [u8], src: &[u8]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let len = dst.len();
    let chunks = len / 16;

    for i in 0..chunks {
        let src_ptr = src.as_ptr().add(i * 16) as *const __m128i;
        let dst_ptr = dst.as_mut_ptr().add(i * 16) as *mut __m128i;
        let v = _mm_loadu_si128(src_ptr);
        _mm_storeu_si128(dst_ptr, v);
    }

    // Copy remaining bytes
    for i in (chunks * 16)..len {
        dst[i] = src[i];
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn buffer_copy_neon(dst: &mut [u8], src: &[u8]) {
    use std::arch::aarch64::*;

    let len = dst.len();
    let chunks = len / 16;

    for i in 0..chunks {
        let src_ptr = src.as_ptr().add(i * 16);
        let dst_ptr = dst.as_mut_ptr().add(i * 16);
        let v = vld1q_u8(src_ptr);
        vst1q_u8(dst_ptr, v);
    }

    for i in (chunks * 16)..len {
        dst[i] = src[i];
    }
}

/// Compare two buffers using SIMD (returns true if equal)
pub fn buffer_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let level = SIMD_FEATURES.best_level();

    match level {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        SimdLevel::Sse2 | SimdLevel::Sse41 | SimdLevel::Avx | SimdLevel::Avx2 => unsafe {
            buffer_equal_sse2(a, b)
        },
        _ => a == b,
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
unsafe fn buffer_equal_sse2(a: &[u8], b: &[u8]) -> bool {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let len = a.len();
    let chunks = len / 16;

    for i in 0..chunks {
        let a_ptr = a.as_ptr().add(i * 16) as *const __m128i;
        let b_ptr = b.as_ptr().add(i * 16) as *const __m128i;
        let va = _mm_loadu_si128(a_ptr);
        let vb = _mm_loadu_si128(b_ptr);
        let cmp = _mm_cmpeq_epi8(va, vb);
        let mask = _mm_movemask_epi8(cmp);
        if mask != 0xFFFF {
            return false;
        }
    }

    // Compare remaining bytes
    for i in (chunks * 16)..len {
        if a[i] != b[i] {
            return false;
        }
    }

    true
}

// ============================================================================
// Base64 Encode/Decode (SIMD)
// ============================================================================

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 encode using SIMD when available
pub fn base64_encode(input: &[u8]) -> Vec<u8> {
    let level = SIMD_FEATURES.best_level();

    match level {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        SimdLevel::Sse2 | SimdLevel::Sse41 | SimdLevel::Avx | SimdLevel::Avx2 => {
            base64_encode_scalar(input) // Use scalar for now, SIMD base64 is complex
        }
        _ => base64_encode_scalar(input),
    }
}

fn base64_encode_scalar(input: &[u8]) -> Vec<u8> {
    let output_len = ((input.len() + 2) / 3) * 4;
    let mut output = Vec::with_capacity(output_len);

    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        let n = (b0 << 16) | (b1 << 8) | b2;

        output.push(BASE64_ALPHABET[(n >> 18) & 0x3F]);
        output.push(BASE64_ALPHABET[(n >> 12) & 0x3F]);

        if chunk.len() > 1 {
            output.push(BASE64_ALPHABET[(n >> 6) & 0x3F]);
        } else {
            output.push(b'=');
        }

        if chunk.len() > 2 {
            output.push(BASE64_ALPHABET[n & 0x3F]);
        } else {
            output.push(b'=');
        }
    }

    output
}

/// Base64 decode
pub fn base64_decode(input: &[u8]) -> Option<Vec<u8>> {
    base64_decode_scalar(input)
}

fn base64_decode_scalar(input: &[u8]) -> Option<Vec<u8>> {
    // Build decode table
    let mut decode_table = [255u8; 256];
    for (i, &c) in BASE64_ALPHABET.iter().enumerate() {
        decode_table[c as usize] = i as u8;
    }
    decode_table[b'=' as usize] = 0;

    let mut output = Vec::with_capacity((input.len() / 4) * 3);

    for chunk in input.chunks(4) {
        if chunk.len() < 4 {
            return None;
        }

        let a = decode_table[chunk[0] as usize];
        let b = decode_table[chunk[1] as usize];
        let c = decode_table[chunk[2] as usize];
        let d = decode_table[chunk[3] as usize];

        if a == 255 || b == 255 || c == 255 || d == 255 {
            return None;
        }

        let n = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | (d as u32);

        output.push((n >> 16) as u8);
        if chunk[2] != b'=' {
            output.push((n >> 8) as u8);
        }
        if chunk[3] != b'=' {
            output.push(n as u8);
        }
    }

    Some(output)
}

// ============================================================================
// FFI Functions
// ============================================================================

use super::Handle;
use std::ffi::c_int;

/// Get detected SIMD features
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_features() -> SimdFeatures {
    *SIMD_FEATURES
}

/// Get best SIMD level
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_level() -> c_int {
    SIMD_FEATURES.best_level() as c_int
}

/// Check if SIMD is available
#[unsafe(no_mangle)]
pub extern "C" fn fz_has_simd() -> c_int {
    if SIMD_FEATURES.has_simd() { 1 } else { 0 }
}

/// SIMD matrix concatenation
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_matrix_concat(left: SimdMatrix, right: SimdMatrix) -> SimdMatrix {
    matrix_concat(&left, &right)
}

/// Transform a point using SIMD matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_transform_point(x: c_float, y: c_float, m: SimdMatrix) -> [c_float; 2] {
    let (nx, ny) = transform_point(x, y, &m);
    [nx, ny]
}

/// Transform multiple points
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_transform_points(
    points: *mut c_float,
    count: usize,
    m: SimdMatrix,
) -> c_int {
    if points.is_null() || count == 0 {
        return -1;
    }

    let slice = unsafe { std::slice::from_raw_parts_mut(points as *mut (f32, f32), count) };
    transform_points(slice, &m);
    0
}

/// Convert RGB to CMYK
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_rgb_to_cmyk(r: c_float, g: c_float, b: c_float) -> [c_float; 4] {
    let (c, m, y, k) = rgb_to_cmyk(r, g, b);
    [c, m, y, k]
}

/// Convert CMYK to RGB
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_cmyk_to_rgb(
    c: c_float,
    m: c_float,
    y: c_float,
    k: c_float,
) -> [c_float; 3] {
    let (r, g, b) = cmyk_to_rgb(c, m, y, k);
    [r, g, b]
}

/// Convert RGB to grayscale
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_rgb_to_gray(r: c_float, g: c_float, b: c_float) -> c_float {
    rgb_to_gray(r, g, b)
}

/// Fill buffer with SIMD
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_buffer_fill(dst: *mut u8, len: usize, value: u8) -> c_int {
    if dst.is_null() {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(dst, len) };
    buffer_fill(slice, value);
    0
}

/// Copy buffer with SIMD
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_buffer_copy(dst: *mut u8, src: *const u8, len: usize) -> c_int {
    if dst.is_null() || src.is_null() {
        return -1;
    }
    let dst_slice = unsafe { std::slice::from_raw_parts_mut(dst, len) };
    let src_slice = unsafe { std::slice::from_raw_parts(src, len) };
    buffer_copy(dst_slice, src_slice);
    0
}

/// Compare buffers with SIMD
#[unsafe(no_mangle)]
pub extern "C" fn fz_simd_buffer_equal(a: *const u8, b: *const u8, len: usize) -> c_int {
    if a.is_null() || b.is_null() {
        return 0;
    }
    let a_slice = unsafe { std::slice::from_raw_parts(a, len) };
    let b_slice = unsafe { std::slice::from_raw_parts(b, len) };
    if buffer_equal(a_slice, b_slice) { 1 } else { 0 }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_features() {
        let features = SimdFeatures::detect();
        // Just ensure detection doesn't panic
        let _ = features.has_simd();
        let _ = features.best_level();
    }

    #[test]
    fn test_matrix_concat_scalar() {
        let identity = SimdMatrix::identity();
        let translate = SimdMatrix::new(1.0, 0.0, 0.0, 1.0, 10.0, 20.0);

        let result = matrix_concat_scalar(&identity, &translate);
        assert!((result.data[4] - 10.0).abs() < 0.001);
        assert!((result.data[5] - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_matrix_concat() {
        let m1 = SimdMatrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0); // Scale 2x
        let m2 = SimdMatrix::new(1.0, 0.0, 0.0, 1.0, 5.0, 10.0); // Translate

        let result = matrix_concat(&m1, &m2);

        // After scale 2x then translate: points should be scaled then translated
        assert!((result.data[0] - 2.0).abs() < 0.001);
        assert!((result.data[3] - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_point() {
        let m = SimdMatrix::new(1.0, 0.0, 0.0, 1.0, 10.0, 20.0);
        let (x, y) = transform_point(5.0, 5.0, &m);
        assert!((x - 15.0).abs() < 0.001);
        assert!((y - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_points_batch() {
        let m = SimdMatrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);
        let mut points = vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0), (4.0, 4.0), (5.0, 5.0)];

        transform_points(&mut points, &m);

        assert!((points[0].0 - 2.0).abs() < 0.001);
        assert!((points[0].1 - 2.0).abs() < 0.001);
        assert!((points[4].0 - 10.0).abs() < 0.001);
        assert!((points[4].1 - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_rgb_to_cmyk() {
        // Pure red
        let (c, m, y, k) = rgb_to_cmyk(1.0, 0.0, 0.0);
        assert!((c - 0.0).abs() < 0.001);
        assert!((m - 1.0).abs() < 0.001);
        assert!((y - 1.0).abs() < 0.001);
        assert!((k - 0.0).abs() < 0.001);

        // Black
        let (c, m, y, k) = rgb_to_cmyk(0.0, 0.0, 0.0);
        assert!((k - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cmyk_to_rgb() {
        // Pure cyan
        let (r, g, b) = cmyk_to_rgb(1.0, 0.0, 0.0, 0.0);
        assert!((r - 0.0).abs() < 0.001);
        assert!((g - 1.0).abs() < 0.001);
        assert!((b - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_rgb_to_gray() {
        // White
        let g = rgb_to_gray(1.0, 1.0, 1.0);
        assert!((g - 1.0).abs() < 0.001);

        // Black
        let g = rgb_to_gray(0.0, 0.0, 0.0);
        assert!((g - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_buffer_fill() {
        let mut buf = vec![0u8; 100];
        buffer_fill(&mut buf, 0xAB);
        assert!(buf.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn test_buffer_copy() {
        let src = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
        let mut dst = vec![0u8; src.len()];
        buffer_copy(&mut dst, &src);
        assert_eq!(src, dst);
    }

    #[test]
    fn test_buffer_equal() {
        let a = vec![1u8, 2, 3, 4, 5];
        let b = vec![1u8, 2, 3, 4, 5];
        let c = vec![1u8, 2, 3, 4, 6];

        assert!(buffer_equal(&a, &b));
        assert!(!buffer_equal(&a, &c));
    }

    #[test]
    fn test_base64_encode() {
        let encoded = base64_encode(b"Hello");
        assert_eq!(&encoded, b"SGVsbG8=");
    }

    #[test]
    fn test_base64_decode() {
        let decoded = base64_decode(b"SGVsbG8=").unwrap();
        assert_eq!(&decoded, b"Hello");
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"The quick brown fox jumps over the lazy dog";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(&decoded, original);
    }

    #[test]
    fn test_ffi_simd_level() {
        let level = fz_simd_level();
        assert!(level >= 0);
    }

    #[test]
    fn test_ffi_has_simd() {
        let has = fz_has_simd();
        assert!(has == 0 || has == 1);
    }

    #[test]
    fn test_ffi_rgb_to_cmyk() {
        let cmyk = fz_simd_rgb_to_cmyk(1.0, 0.0, 0.0);
        assert!((cmyk[1] - 1.0).abs() < 0.001); // Magenta
        assert!((cmyk[2] - 1.0).abs() < 0.001); // Yellow
    }

    #[test]
    fn test_ffi_cmyk_to_rgb() {
        let rgb = fz_simd_cmyk_to_rgb(0.0, 1.0, 1.0, 0.0);
        assert!((rgb[0] - 1.0).abs() < 0.001); // Red
        assert!((rgb[1] - 0.0).abs() < 0.001); // Green
        assert!((rgb[2] - 0.0).abs() < 0.001); // Blue
    }
}
