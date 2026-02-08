//! GPU Types and Structures
//!
//! Common types used across all GPU backends.

use super::super::Handle;
// Note: Matrix is imported from crate::fitz::geometry when needed

// ============================================================================
// Enums
// ============================================================================

/// GPU backend types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuBackendType {
    /// Automatically select the best available backend
    Auto = 0,
    /// OpenGL (2.1+ / ES 2.0+)
    OpenGL = 1,
    /// Vulkan (1.0+)
    Vulkan = 2,
    /// Metal (macOS/iOS)
    Metal = 3,
    /// DirectX 11
    DirectX11 = 4,
    /// DirectX 12
    DirectX12 = 5,
}

/// Texture/pixel formats
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuFormat {
    /// 8-bit RGBA (32 bits per pixel)
    Rgba8 = 0,
    /// 8-bit BGRA (32 bits per pixel)
    Bgra8 = 1,
    /// 8-bit RGB (24 bits per pixel)
    Rgb8 = 2,
    /// 8-bit single channel (8 bits per pixel)
    R8 = 3,
    /// 16-bit float RGBA (64 bits per pixel)
    Rgba16f = 4,
    /// 32-bit float RGBA (128 bits per pixel)
    Rgba32f = 5,
}

impl GpuFormat {
    /// Get bytes per pixel for this format
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            GpuFormat::Rgba8 | GpuFormat::Bgra8 => 4,
            GpuFormat::Rgb8 => 3,
            GpuFormat::R8 => 1,
            GpuFormat::Rgba16f => 8,
            GpuFormat::Rgba32f => 16,
        }
    }

    /// Get number of components
    pub fn components(&self) -> usize {
        match self {
            GpuFormat::Rgba8 | GpuFormat::Bgra8 | GpuFormat::Rgba16f | GpuFormat::Rgba32f => 4,
            GpuFormat::Rgb8 => 3,
            GpuFormat::R8 => 1,
        }
    }
}

/// Blend modes for compositing
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GpuBlendMode {
    /// Normal alpha blending
    #[default]
    Normal = 0,
    /// Multiply blend mode
    Multiply = 1,
    /// Screen blend mode
    Screen = 2,
    /// Overlay blend mode
    Overlay = 3,
    /// Darken blend mode
    Darken = 4,
    /// Lighten blend mode
    Lighten = 5,
    /// Color dodge
    ColorDodge = 6,
    /// Color burn
    ColorBurn = 7,
    /// Hard light
    HardLight = 8,
    /// Soft light
    SoftLight = 9,
    /// Difference
    Difference = 10,
    /// Exclusion
    Exclusion = 11,
}

/// GPU buffer usage types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuBufferUsage {
    /// Vertex data
    Vertex = 0,
    /// Index data
    Index = 1,
    /// Uniform/constant data
    Uniform = 2,
    /// General storage (SSBO)
    Storage = 3,
}

/// Shader types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuShaderType {
    /// Vertex shader
    Vertex = 0,
    /// Fragment/pixel shader
    Fragment = 1,
    /// Compute shader
    Compute = 2,
}

/// Texture filtering modes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GpuFilter {
    /// Nearest neighbor (pixelated)
    Nearest = 0,
    /// Linear interpolation (smooth)
    #[default]
    Linear = 1,
}

/// Texture wrap modes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GpuWrap {
    /// Clamp to edge
    #[default]
    ClampToEdge = 0,
    /// Repeat
    Repeat = 1,
    /// Mirrored repeat
    MirroredRepeat = 2,
}

// ============================================================================
// Structures
// ============================================================================

/// GPU texture
#[derive(Debug)]
pub struct GpuTexture {
    /// Native texture handle (backend-specific)
    pub native_handle: u64,
    /// Texture width in pixels
    pub width: u32,
    /// Texture height in pixels
    pub height: u32,
    /// Pixel format
    pub format: GpuFormat,
    /// Minification filter
    pub min_filter: GpuFilter,
    /// Magnification filter
    pub mag_filter: GpuFilter,
    /// Horizontal wrap mode
    pub wrap_s: GpuWrap,
    /// Vertical wrap mode
    pub wrap_t: GpuWrap,
    /// Backend type that created this texture
    pub backend: GpuBackendType,
}

impl GpuTexture {
    /// Create a new texture descriptor
    pub fn new(width: u32, height: u32, format: GpuFormat, backend: GpuBackendType) -> Self {
        Self {
            native_handle: 0,
            width,
            height,
            format,
            min_filter: GpuFilter::Linear,
            mag_filter: GpuFilter::Linear,
            wrap_s: GpuWrap::ClampToEdge,
            wrap_t: GpuWrap::ClampToEdge,
            backend,
        }
    }

    /// Get size in bytes
    pub fn size_bytes(&self) -> usize {
        (self.width as usize) * (self.height as usize) * self.format.bytes_per_pixel()
    }
}

/// GPU shader program
#[derive(Debug)]
pub struct GpuShader {
    /// Native shader handle (backend-specific)
    pub native_handle: u64,
    /// Shader name/identifier
    pub name: String,
    /// Backend type
    pub backend: GpuBackendType,
}

impl GpuShader {
    /// Create a new shader descriptor
    pub fn new(name: impl Into<String>, backend: GpuBackendType) -> Self {
        Self {
            native_handle: 0,
            name: name.into(),
            backend,
        }
    }
}

/// GPU buffer
#[derive(Debug)]
pub struct GpuBuffer {
    /// Native buffer handle (backend-specific)
    pub native_handle: u64,
    /// Buffer size in bytes
    pub size: usize,
    /// Buffer usage
    pub usage: GpuBufferUsage,
    /// Backend type
    pub backend: GpuBackendType,
}

impl GpuBuffer {
    /// Create a new buffer descriptor
    pub fn new(size: usize, usage: GpuBufferUsage, backend: GpuBackendType) -> Self {
        Self {
            native_handle: 0,
            size,
            usage,
            backend,
        }
    }
}

/// GPU device capabilities
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    /// Maximum texture dimension (width or height)
    pub max_texture_size: u32,
    /// Maximum number of texture units
    pub max_texture_units: u32,
    /// Supports compute shaders
    pub compute_shaders: bool,
    /// Supports geometry shaders
    pub geometry_shaders: bool,
    /// Supports tessellation
    pub tessellation: bool,
    /// Maximum MSAA samples
    pub max_msaa_samples: u32,
    /// Supports float textures
    pub float_textures: bool,
    /// Supports instanced rendering
    pub instancing: bool,
    /// Available VRAM in MB (0 if unknown)
    pub vram_mb: u32,
    /// Device name
    pub device_name: String,
    /// Vendor name
    pub vendor_name: String,
    /// Driver version
    pub driver_version: String,
}

impl Default for GpuCapabilities {
    fn default() -> Self {
        Self {
            max_texture_size: 4096,
            max_texture_units: 8,
            compute_shaders: false,
            geometry_shaders: false,
            tessellation: false,
            max_msaa_samples: 1,
            float_textures: false,
            instancing: false,
            vram_mb: 0,
            device_name: String::new(),
            vendor_name: String::new(),
            driver_version: String::new(),
        }
    }
}

/// Render pass descriptor
#[derive(Debug, Clone)]
pub struct GpuRenderPass {
    /// Target texture handles
    pub color_attachments: Vec<Handle>,
    /// Depth texture handle (0 if none)
    pub depth_attachment: Handle,
    /// Clear color (RGBA)
    pub clear_color: [f32; 4],
    /// Clear depth value
    pub clear_depth: f32,
    /// Whether to clear color
    pub clear_color_enabled: bool,
    /// Whether to clear depth
    pub clear_depth_enabled: bool,
}

impl Default for GpuRenderPass {
    fn default() -> Self {
        Self {
            color_attachments: Vec::new(),
            depth_attachment: 0,
            clear_color: [1.0, 1.0, 1.0, 1.0],
            clear_depth: 1.0,
            clear_color_enabled: true,
            clear_depth_enabled: true,
        }
    }
}

/// Viewport
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuViewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl GpuViewport {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

/// Scissor rect
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuScissor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// ============================================================================
// Error Types
// ============================================================================

/// GPU error type
#[derive(Debug, Clone)]
pub enum GpuError {
    /// Backend not available on this system
    BackendNotAvailable(GpuBackendType),
    /// Failed to initialize GPU
    InitializationFailed(String),
    /// Shader compilation failed
    ShaderCompilationFailed(String),
    /// Out of GPU memory
    OutOfMemory,
    /// Invalid operation
    InvalidOperation(String),
    /// Feature not supported
    NotSupported(String),
    /// Device lost (GPU reset)
    DeviceLost,
    /// Generic error
    Other(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::BackendNotAvailable(b) => write!(f, "GPU backend {:?} not available", b),
            GpuError::InitializationFailed(s) => write!(f, "GPU initialization failed: {}", s),
            GpuError::ShaderCompilationFailed(s) => write!(f, "Shader compilation failed: {}", s),
            GpuError::OutOfMemory => write!(f, "Out of GPU memory"),
            GpuError::InvalidOperation(s) => write!(f, "Invalid GPU operation: {}", s),
            GpuError::NotSupported(s) => write!(f, "GPU feature not supported: {}", s),
            GpuError::DeviceLost => write!(f, "GPU device lost"),
            GpuError::Other(s) => write!(f, "GPU error: {}", s),
        }
    }
}

impl std::error::Error for GpuError {}

/// Result type for GPU operations
pub type GpuResult<T> = Result<T, GpuError>;
