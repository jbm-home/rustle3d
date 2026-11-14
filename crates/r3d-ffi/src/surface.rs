//! Wrapping a texture owned by the UI compositor as a wgpu render target.
//!
//! macOS: the Swift side creates an IOSurface, registers it with Flutter as
//! an external texture and passes its pointer here. We create a Metal
//! texture backed by the same memory on wgpu's device, so what we render is
//! composited by Flutter without any copy. Other platforms come later
//! (D3D12 shared handles, Vulkan dma-buf).

/// Wrap the IOSurface at `surface` (an `IOSurfaceRef` pointer from this
/// process) as a `Bgra8Unorm` render target of `width × height`.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub(crate) fn wrap_ui_surface(
    device: &wgpu::Device,
    surface: u64,
    width: u32,
    height: u32,
) -> Result<wgpu::Texture, String> {
    use std::ptr::NonNull;

    use objc2_core_foundation::CFRetained;
    use objc2_io_surface::IOSurfaceRef;
    use objc2_metal::{
        MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTextureDescriptor, MTLTextureType,
        MTLTextureUsage,
    };
    use wgpu::hal::api::Metal;

    let ptr = NonNull::new(surface as usize as *mut IOSurfaceRef)
        .ok_or_else(|| "null IOSurface pointer".to_owned())?;
    // SAFETY: `surface` is an IOSurfaceRef created by the Swift side of this
    // process, which keeps it alive while the Flutter texture is registered.
    // We take our own +1 reference, released when `retained` drops below
    // (the Metal texture keeps the IOSurface alive on its own).
    let retained: CFRetained<IOSurfaceRef> = unsafe { CFRetained::retain(ptr) };

    // SAFETY: we only read the raw Metal device to create a texture on it;
    // the hal guard does not outlive this function.
    let hal_device = unsafe { device.as_hal::<Metal>() }
        .ok_or_else(|| "the GPU device is not a Metal device".to_owned())?;
    let desc = MTLTextureDescriptor::new();
    desc.setTextureType(MTLTextureType::Type2D);
    desc.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
    // SAFETY: plain setters on a fresh descriptor; sizes are in range
    // (Flutter only creates textures within the window size).
    unsafe {
        desc.setWidth(width as usize);
        desc.setHeight(height as usize);
        desc.setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        desc.setStorageMode(MTLStorageMode::Shared);
    }
    let raw = hal_device
        .raw_device()
        .newTextureWithDescriptor_iosurface_plane(&desc, &retained, 0)
        .ok_or_else(|| "Metal refused the IOSurface texture".to_owned())?;

    let copy_size = wgpu::hal::CopyExtent {
        width,
        height,
        depth: 1,
    };
    // SAFETY: `raw` is a 2D, single-mip, BGRA8 texture of exactly
    // `width × height`, matching the format and size we declare.
    let hal_texture = unsafe {
        wgpu::hal::metal::Device::texture_from_raw(
            raw,
            wgpu::TextureFormat::Bgra8Unorm,
            MTLTextureType::Type2D,
            1,
            1,
            copy_size,
            None,
        )
    };
    drop(hal_device);
    // SAFETY: the hal texture was created on this device with the
    // descriptor below.
    let texture = unsafe {
        device.create_texture_from_hal::<Metal>(
            hal_texture,
            &wgpu::TextureDescriptor {
                label: Some("ui surface"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            wgpu::TextureUses::UNINITIALIZED,
        )
    };
    Ok(texture)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn wrap_ui_surface(
    _device: &wgpu::Device,
    _surface: u64,
    _width: u32,
    _height: u32,
) -> Result<wgpu::Texture, String> {
    Err("shared UI textures are only implemented on macOS for now".to_owned())
}
