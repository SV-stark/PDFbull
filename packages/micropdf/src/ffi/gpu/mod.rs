//! GPU Acceleration Module
//!
//! This module provides GPU-accelerated rendering backends for MicroPDF.
//! Supports multiple graphics APIs:
//! - OpenGL (cross-platform)
//! - Vulkan (cross-platform, modern)
//! - Metal (macOS/iOS)
//! - DirectX 11/12 (Windows)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    GpuDevice Trait                       │
//! │  (create_texture, render_page, composite, etc.)         │
//! └─────────────────────────────────────────────────────────┘
//!          │              │              │              │
//!          ▼              ▼              ▼              ▼
//!    ┌─────────┐   ┌──────────┐   ┌─────────┐   ┌──────────┐
//!    │ OpenGL  │   │  Vulkan  │   │  Metal  │   │ DirectX  │
//!    │ Backend │   │  Backend │   │ Backend │   │ Backend  │
//!    └─────────┘   └──────────┘   └─────────┘   └──────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use micropdf::ffi::gpu::{GpuDevice, GpuBackend, create_device};
//!
//! // Auto-select best available backend
//! let device = create_device(GpuBackend::Auto)?;
//!
//! // Or specify a backend
//! let device = create_device(GpuBackend::Vulkan)?;
//!
//! // Create a texture for rendering
//! let texture = device.create_texture(width, height, GpuFormat::Rgba8)?;
//!
//! // Render a page to the texture
//! device.render_page(&page, &texture, &matrix)?;
//! ```

pub mod backend;
pub mod opengl;
pub mod vulkan;

#[cfg(target_os = "macos")]
pub mod metal;

#[cfg(target_os = "windows")]
pub mod directx;

mod types;

pub use backend::*;
pub use types::*;

use super::{Handle, HandleStore};
use crate::fitz::geometry::Matrix;
use std::sync::LazyLock;

// ============================================================================
// Handle Stores
// ============================================================================

/// GPU device handles
pub static GPU_DEVICES: LazyLock<HandleStore<Box<dyn GpuDevice + Send + Sync>>> =
    LazyLock::new(HandleStore::new);

/// GPU texture handles
pub static GPU_TEXTURES: LazyLock<HandleStore<GpuTexture>> = LazyLock::new(HandleStore::new);

/// GPU shader handles
pub static GPU_SHADERS: LazyLock<HandleStore<GpuShader>> = LazyLock::new(HandleStore::new);

/// GPU buffer handles
pub static GPU_BUFFERS: LazyLock<HandleStore<GpuBuffer>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Device Management
// ============================================================================

/// Create a GPU device with the specified backend
///
/// # Arguments
/// * `backend` - The GPU backend to use (0=Auto, 1=OpenGL, 2=Vulkan, 3=Metal, 4=DirectX)
///
/// # Returns
/// Handle to the GPU device, or 0 on failure
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_create_device(backend: i32) -> Handle {
    let backend_type = match backend {
        0 => GpuBackendType::Auto,
        1 => GpuBackendType::OpenGL,
        2 => GpuBackendType::Vulkan,
        3 => GpuBackendType::Metal,
        4 => GpuBackendType::DirectX11,
        5 => GpuBackendType::DirectX12,
        _ => return 0,
    };

    match create_device(backend_type) {
        Ok(device) => GPU_DEVICES.insert(device),
        Err(_) => 0,
    }
}

/// Drop a GPU device
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_drop_device(device: Handle) {
    GPU_DEVICES.remove(device);
}

/// Get the backend type of a device
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_device_backend(device: Handle) -> i32 {
    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            return match guard.backend() {
                GpuBackendType::Auto => 0,
                GpuBackendType::OpenGL => 1,
                GpuBackendType::Vulkan => 2,
                GpuBackendType::Metal => 3,
                GpuBackendType::DirectX11 => 4,
                GpuBackendType::DirectX12 => 5,
            };
        }
    }
    -1
}

/// Check if a backend is available on this system
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_backend_available(backend: i32) -> i32 {
    let backend_type = match backend {
        1 => GpuBackendType::OpenGL,
        2 => GpuBackendType::Vulkan,
        3 => GpuBackendType::Metal,
        4 => GpuBackendType::DirectX11,
        5 => GpuBackendType::DirectX12,
        _ => return 0,
    };

    i32::from(is_backend_available(backend_type))
}

// ============================================================================
// FFI Functions - Texture Management
// ============================================================================

/// Create a GPU texture
///
/// # Arguments
/// * `device` - GPU device handle
/// * `width` - Texture width in pixels
/// * `height` - Texture height in pixels
/// * `format` - Pixel format (0=RGBA8, 1=BGRA8, 2=RGB8, 3=R8)
///
/// # Returns
/// Handle to the texture, or 0 on failure
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_create_texture(
    device: Handle,
    width: i32,
    height: i32,
    format: i32,
) -> Handle {
    let fmt = match format {
        0 => GpuFormat::Rgba8,
        1 => GpuFormat::Bgra8,
        2 => GpuFormat::Rgb8,
        3 => GpuFormat::R8,
        4 => GpuFormat::Rgba16f,
        5 => GpuFormat::Rgba32f,
        _ => return 0,
    };

    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if let Ok(texture) = guard.create_texture(width as u32, height as u32, fmt) {
                return GPU_TEXTURES.insert(texture);
            }
        }
    }
    0
}

/// Drop a GPU texture
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_drop_texture(texture: Handle) {
    GPU_TEXTURES.remove(texture);
}

/// Get texture width
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_texture_width(texture: Handle) -> i32 {
    if let Some(tex) = GPU_TEXTURES.get(texture) {
        if let Ok(guard) = tex.lock() {
            return guard.width as i32;
        }
    }
    0
}

/// Get texture height
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_texture_height(texture: Handle) -> i32 {
    if let Some(tex) = GPU_TEXTURES.get(texture) {
        if let Ok(guard) = tex.lock() {
            return guard.height as i32;
        }
    }
    0
}

/// Upload data to a texture
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `width * height * bytes_per_pixel` bytes
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_gpu_texture_upload(
    device: Handle,
    texture: Handle,
    data: *const u8,
    stride: i32,
) -> i32 {
    if data.is_null() {
        return -1;
    }

    let (width, height, format) = if let Some(tex) = GPU_TEXTURES.get(texture) {
        if let Ok(guard) = tex.lock() {
            (guard.width, guard.height, guard.format)
        } else {
            return -1;
        }
    } else {
        return -1;
    };

    let bytes_per_pixel = format.bytes_per_pixel();
    let data_size = if stride > 0 {
        (stride as usize) * (height as usize)
    } else {
        (width as usize) * bytes_per_pixel * (height as usize)
    };

    // SAFETY: Caller guarantees data is valid
    let slice = unsafe { std::slice::from_raw_parts(data, data_size) };

    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if let Some(tex) = GPU_TEXTURES.get(texture) {
                if let Ok(mut tex_guard) = tex.lock() {
                    if guard
                        .upload_texture(&mut tex_guard, slice, stride as u32)
                        .is_ok()
                    {
                        return 0;
                    }
                }
            }
        }
    }
    -1
}

/// Download texture data to CPU memory
///
/// # Safety
/// Caller must ensure `data` points to writable memory of sufficient size
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_gpu_texture_download(
    device: Handle,
    texture: Handle,
    data: *mut u8,
    stride: i32,
) -> i32 {
    if data.is_null() {
        return -1;
    }

    let (width, height, format) = if let Some(tex) = GPU_TEXTURES.get(texture) {
        if let Ok(guard) = tex.lock() {
            (guard.width, guard.height, guard.format)
        } else {
            return -1;
        }
    } else {
        return -1;
    };

    let bytes_per_pixel = format.bytes_per_pixel();
    let data_size = if stride > 0 {
        (stride as usize) * (height as usize)
    } else {
        (width as usize) * bytes_per_pixel * (height as usize)
    };

    // SAFETY: Caller guarantees data is valid writable memory
    let slice = unsafe { std::slice::from_raw_parts_mut(data, data_size) };

    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if let Some(tex) = GPU_TEXTURES.get(texture) {
                if let Ok(tex_guard) = tex.lock() {
                    if guard
                        .download_texture(&tex_guard, slice, stride as u32)
                        .is_ok()
                    {
                        return 0;
                    }
                }
            }
        }
    }
    -1
}

// ============================================================================
// FFI Functions - Rendering
// ============================================================================

/// Render a page to a GPU texture
///
/// # Arguments
/// * `device` - GPU device handle
/// * `page` - Page handle to render
/// * `texture` - Target texture handle
/// * `ctm` - Transformation matrix (6 floats: a, b, c, d, e, f)
///
/// # Returns
/// 0 on success, negative on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_render_page(
    device: Handle,
    page: Handle,
    texture: Handle,
    ctm: *const f32,
) -> i32 {
    if ctm.is_null() {
        return -1;
    }

    // SAFETY: Caller guarantees ctm points to 6 floats
    let matrix = unsafe {
        Matrix::new(
            *ctm,
            *ctm.add(1),
            *ctm.add(2),
            *ctm.add(3),
            *ctm.add(4),
            *ctm.add(5),
        )
    };

    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if let Some(tex) = GPU_TEXTURES.get(texture) {
                if let Ok(mut tex_guard) = tex.lock() {
                    if guard.render_page(page, &mut tex_guard, &matrix).is_ok() {
                        return 0;
                    }
                }
            }
        }
    }
    -1
}

/// Clear a texture with a color
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_clear_texture(
    device: Handle,
    texture: Handle,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) -> i32 {
    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if let Some(tex) = GPU_TEXTURES.get(texture) {
                if let Ok(mut tex_guard) = tex.lock() {
                    if guard.clear_texture(&mut tex_guard, [r, g, b, a]).is_ok() {
                        return 0;
                    }
                }
            }
        }
    }
    -1
}

/// Composite one texture onto another
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_composite(
    device: Handle,
    src: Handle,
    dst: Handle,
    x: i32,
    y: i32,
    blend_mode: i32,
) -> i32 {
    let mode = match blend_mode {
        0 => GpuBlendMode::Normal,
        1 => GpuBlendMode::Multiply,
        2 => GpuBlendMode::Screen,
        3 => GpuBlendMode::Overlay,
        4 => GpuBlendMode::Darken,
        5 => GpuBlendMode::Lighten,
        _ => GpuBlendMode::Normal,
    };

    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if let Some(src_tex) = GPU_TEXTURES.get(src) {
                if let Some(dst_tex) = GPU_TEXTURES.get(dst) {
                    if let Ok(src_guard) = src_tex.lock() {
                        if let Ok(mut dst_guard) = dst_tex.lock() {
                            if guard
                                .composite(&src_guard, &mut dst_guard, x, y, mode)
                                .is_ok()
                            {
                                return 0;
                            }
                        }
                    }
                }
            }
        }
    }
    -1
}

// ============================================================================
// FFI Functions - Shader Management
// ============================================================================

/// Create a shader program
///
/// # Safety
/// Caller must ensure `vertex_src` and `fragment_src` are valid null-terminated strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_gpu_create_shader(
    device: Handle,
    vertex_src: *const std::ffi::c_char,
    fragment_src: *const std::ffi::c_char,
) -> Handle {
    if vertex_src.is_null() || fragment_src.is_null() {
        return 0;
    }

    let vertex = match unsafe { std::ffi::CStr::from_ptr(vertex_src) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let fragment = match unsafe { std::ffi::CStr::from_ptr(fragment_src) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if let Ok(shader) = guard.create_shader(vertex, fragment) {
                return GPU_SHADERS.insert(shader);
            }
        }
    }
    0
}

/// Drop a shader
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_drop_shader(shader: Handle) {
    GPU_SHADERS.remove(shader);
}

// ============================================================================
// FFI Functions - Buffer Management
// ============================================================================

/// Create a GPU buffer
///
/// # Arguments
/// * `device` - GPU device handle
/// * `size` - Buffer size in bytes
/// * `usage` - Buffer usage (0=Vertex, 1=Index, 2=Uniform, 3=Storage)
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_create_buffer(device: Handle, size: i32, usage: i32) -> Handle {
    let buffer_usage = match usage {
        0 => GpuBufferUsage::Vertex,
        1 => GpuBufferUsage::Index,
        2 => GpuBufferUsage::Uniform,
        3 => GpuBufferUsage::Storage,
        _ => return 0,
    };

    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if let Ok(buffer) = guard.create_buffer(size as usize, buffer_usage) {
                return GPU_BUFFERS.insert(buffer);
            }
        }
    }
    0
}

/// Drop a GPU buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_drop_buffer(buffer: Handle) {
    GPU_BUFFERS.remove(buffer);
}

// ============================================================================
// FFI Functions - Synchronization
// ============================================================================

/// Flush all pending GPU commands
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_flush(device: Handle) -> i32 {
    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if guard.flush().is_ok() {
                return 0;
            }
        }
    }
    -1
}

/// Wait for all GPU commands to complete
#[unsafe(no_mangle)]
pub extern "C" fn fz_gpu_finish(device: Handle) -> i32 {
    if let Some(dev) = GPU_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if guard.finish().is_ok() {
                return 0;
            }
        }
    }
    -1
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_availability() {
        // At least one backend should report its availability status
        let _opengl = fz_gpu_backend_available(1);
        let _vulkan = fz_gpu_backend_available(2);
    }

    #[test]
    fn test_invalid_backend() {
        assert_eq!(fz_gpu_backend_available(99), 0);
    }
}
