//! GPU Backend Trait and Factory
//!
//! Defines the common interface for all GPU backends and provides
//! factory functions for creating devices.

use super::super::Handle;
use super::types::*;
use crate::fitz::geometry::Matrix;

// ============================================================================
// GpuDevice Trait
// ============================================================================

/// Trait implemented by all GPU backends
///
/// This provides a unified interface for GPU operations across
/// OpenGL, Vulkan, Metal, and DirectX.
pub trait GpuDevice: Send + Sync {
    // ========================================================================
    // Device Info
    // ========================================================================

    /// Get the backend type
    fn backend(&self) -> GpuBackendType;

    /// Get device capabilities
    fn capabilities(&self) -> &GpuCapabilities;

    /// Get device name
    fn name(&self) -> &str {
        &self.capabilities().device_name
    }

    // ========================================================================
    // Texture Operations
    // ========================================================================

    /// Create a new texture
    fn create_texture(&self, width: u32, height: u32, format: GpuFormat) -> GpuResult<GpuTexture>;

    /// Destroy a texture (releases GPU resources)
    fn destroy_texture(&self, texture: &GpuTexture) -> GpuResult<()>;

    /// Upload data to a texture
    fn upload_texture(&self, texture: &mut GpuTexture, data: &[u8], stride: u32) -> GpuResult<()>;

    /// Download texture data to CPU memory
    fn download_texture(&self, texture: &GpuTexture, data: &mut [u8], stride: u32)
    -> GpuResult<()>;

    /// Clear a texture with a color
    fn clear_texture(&self, texture: &mut GpuTexture, color: [f32; 4]) -> GpuResult<()>;

    // ========================================================================
    // Shader Operations
    // ========================================================================

    /// Create a shader program from source
    fn create_shader(&self, vertex_src: &str, fragment_src: &str) -> GpuResult<GpuShader>;

    /// Destroy a shader
    fn destroy_shader(&self, shader: &GpuShader) -> GpuResult<()>;

    // ========================================================================
    // Buffer Operations
    // ========================================================================

    /// Create a GPU buffer
    fn create_buffer(&self, size: usize, usage: GpuBufferUsage) -> GpuResult<GpuBuffer>;

    /// Destroy a buffer
    fn destroy_buffer(&self, buffer: &GpuBuffer) -> GpuResult<()>;

    /// Upload data to a buffer
    fn upload_buffer(&self, buffer: &mut GpuBuffer, data: &[u8], offset: usize) -> GpuResult<()>;

    // ========================================================================
    // Rendering Operations
    // ========================================================================

    /// Render a PDF page to a texture
    fn render_page(
        &self,
        page: Handle,
        texture: &mut GpuTexture,
        transform: &Matrix,
    ) -> GpuResult<()>;

    /// Composite one texture onto another
    fn composite(
        &self,
        src: &GpuTexture,
        dst: &mut GpuTexture,
        x: i32,
        y: i32,
        blend_mode: GpuBlendMode,
    ) -> GpuResult<()>;

    /// Draw a textured quad
    fn draw_quad(
        &self,
        texture: &GpuTexture,
        dst: &mut GpuTexture,
        src_rect: [f32; 4],
        dst_rect: [f32; 4],
        color: [f32; 4],
    ) -> GpuResult<()>;

    // ========================================================================
    // Synchronization
    // ========================================================================

    /// Flush pending commands (non-blocking)
    fn flush(&self) -> GpuResult<()>;

    /// Wait for all commands to complete (blocking)
    fn finish(&self) -> GpuResult<()>;
}

// ============================================================================
// Backend Availability
// ============================================================================

/// Check if a backend is available on this system
pub fn is_backend_available(backend: GpuBackendType) -> bool {
    match backend {
        GpuBackendType::Auto => {
            // Check in order of preference
            is_backend_available(GpuBackendType::Vulkan)
                || is_backend_available(GpuBackendType::Metal)
                || is_backend_available(GpuBackendType::DirectX12)
                || is_backend_available(GpuBackendType::OpenGL)
        }
        GpuBackendType::OpenGL => {
            // OpenGL is almost always available
            cfg!(any(
                target_os = "linux",
                target_os = "windows",
                target_os = "macos"
            ))
        }
        GpuBackendType::Vulkan => {
            // Vulkan availability depends on drivers
            cfg!(any(
                target_os = "linux",
                target_os = "windows",
                target_os = "android"
            ))
        }
        GpuBackendType::Metal => {
            // Metal is only on Apple platforms
            cfg!(any(target_os = "macos", target_os = "ios"))
        }
        GpuBackendType::DirectX11 | GpuBackendType::DirectX12 => {
            // DirectX is Windows only
            cfg!(target_os = "windows")
        }
    }
}

/// Get the best available backend for this system
pub fn best_backend() -> GpuBackendType {
    #[cfg(target_os = "macos")]
    {
        GpuBackendType::Metal
    }

    #[cfg(target_os = "windows")]
    {
        if is_backend_available(GpuBackendType::DirectX12) {
            GpuBackendType::DirectX12
        } else if is_backend_available(GpuBackendType::Vulkan) {
            GpuBackendType::Vulkan
        } else {
            GpuBackendType::OpenGL
        }
    }

    #[cfg(target_os = "linux")]
    {
        if is_backend_available(GpuBackendType::Vulkan) {
            GpuBackendType::Vulkan
        } else {
            GpuBackendType::OpenGL
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        GpuBackendType::OpenGL
    }
}

// ============================================================================
// Device Factory
// ============================================================================

/// Create a GPU device with the specified backend
pub fn create_device(backend: GpuBackendType) -> GpuResult<Box<dyn GpuDevice + Send + Sync>> {
    let actual_backend = if backend == GpuBackendType::Auto {
        best_backend()
    } else {
        backend
    };

    if !is_backend_available(actual_backend) {
        return Err(GpuError::BackendNotAvailable(actual_backend));
    }

    match actual_backend {
        GpuBackendType::Auto => unreachable!(),
        GpuBackendType::OpenGL => Ok(Box::new(super::opengl::OpenGLDevice::new()?)),
        GpuBackendType::Vulkan => Ok(Box::new(super::vulkan::VulkanDevice::new()?)),
        #[cfg(target_os = "macos")]
        GpuBackendType::Metal => Ok(Box::new(super::metal::MetalDevice::new()?)),
        #[cfg(not(target_os = "macos"))]
        GpuBackendType::Metal => Err(GpuError::BackendNotAvailable(GpuBackendType::Metal)),
        #[cfg(target_os = "windows")]
        GpuBackendType::DirectX11 => Ok(Box::new(super::directx::DirectX11Device::new()?)),
        #[cfg(target_os = "windows")]
        GpuBackendType::DirectX12 => Ok(Box::new(super::directx::DirectX12Device::new()?)),
        #[cfg(not(target_os = "windows"))]
        GpuBackendType::DirectX11 | GpuBackendType::DirectX12 => {
            Err(GpuError::BackendNotAvailable(actual_backend))
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
    fn test_best_backend() {
        let backend = best_backend();
        assert!(is_backend_available(backend));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(GpuFormat::Rgba8.bytes_per_pixel(), 4);
        assert_eq!(GpuFormat::Rgb8.bytes_per_pixel(), 3);
        assert_eq!(GpuFormat::R8.bytes_per_pixel(), 1);
        assert_eq!(GpuFormat::Rgba16f.bytes_per_pixel(), 8);
        assert_eq!(GpuFormat::Rgba32f.bytes_per_pixel(), 16);
    }
}
