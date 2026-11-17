//! Wrapping a texture owned by the UI compositor as a wgpu render target.
//!
//! The UI side creates a GPU texture, registers it with Flutter as an
//! external texture and passes its handle here. We open the same memory on
//! wgpu's device, so what we render is composited by Flutter without any
//! copy.
//!
//! - macOS: an `IOSurfaceRef` pointer, wrapped as a Metal texture.
//! - Windows: the NT handle of a D3D11 texture created on Flutter's
//!   adapter, opened as a D3D12 resource. The device must live on that
//!   adapter, so [`ui_gpu`] picks it by the LUID the runner reports.
//!
//! Elsewhere (and whenever wrapping fails) the UI reads the pixels back.

use std::sync::Mutex;

use r3d_render::Gpu;

/// The adapter the UI composites on (Windows: its LUID), set by the UI
/// before the first view.
static UI_ADAPTER: Mutex<Option<i64>> = Mutex::new(None);

/// Remember the adapter the UI composites on.
pub(crate) fn set_ui_adapter(luid: i64) {
    if let Ok(mut slot) = UI_ADAPTER.lock() {
        *slot = Some(luid);
    }
}

/// The GPU for the views: the UI's adapter when it is known and found,
/// otherwise the default one.
pub(crate) fn ui_gpu() -> Result<Gpu, r3d_render::RenderError> {
    let luid = UI_ADAPTER.lock().ok().and_then(|slot| *slot);
    match luid.and_then(gpu_on_adapter) {
        Some(gpu) => Ok(gpu),
        None => Gpu::headless(),
    }
}

/// The texture descriptor of a UI surface of `width × height`.
#[cfg(any(target_os = "macos", windows))]
fn descriptor(width: u32, height: u32) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
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
    }
}

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
            &descriptor(width, height),
            wgpu::TextureUses::UNINITIALIZED,
        )
    };
    Ok(texture)
}

/// Open the shared D3D11 texture behind the NT handle `surface` (from the
/// runner, same process) as a `Bgra8Unorm` render target of
/// `width × height`. Fails when the device is not on the texture's adapter.
#[cfg(windows)]
#[allow(unsafe_code)]
pub(crate) fn wrap_ui_surface(
    device: &wgpu::Device,
    surface: u64,
    width: u32,
    height: u32,
) -> Result<wgpu::Texture, String> {
    use wgpu::hal::api::Dx12;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Graphics::Direct3D12::{
        D3D12_RESOURCE_DIMENSION_TEXTURE2D, ID3D12Resource,
    };
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

    let handle = HANDLE(surface as usize as *mut std::ffi::c_void);
    if handle.is_invalid() {
        return Err("invalid shared texture handle".to_owned());
    }
    // SAFETY: we only read the raw D3D12 device to open a resource on it;
    // the hal guard does not outlive this function.
    let hal_device = unsafe { device.as_hal::<Dx12>() }
        .ok_or_else(|| "the GPU device is not a D3D12 device".to_owned())?;
    let mut resource: Option<ID3D12Resource> = None;
    // SAFETY: `handle` is an NT handle the runner created for a shared
    // texture and keeps open while the Flutter texture is registered; the
    // resource takes its own reference to the memory.
    unsafe {
        hal_device
            .raw_device()
            .OpenSharedHandle(handle, &mut resource)
    }
    .map_err(|e| format!("cannot open the shared texture: {e}"))?;
    let resource = resource.ok_or_else(|| "no shared texture behind the handle".to_owned())?;
    // SAFETY: a plain query on a live resource.
    let desc = unsafe { resource.GetDesc() };
    if desc.Dimension != D3D12_RESOURCE_DIMENSION_TEXTURE2D
        || desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM
        || desc.Width != u64::from(width)
        || desc.Height != height
        || desc.MipLevels != 1
        || desc.SampleDesc.Count != 1
    {
        return Err("the shared texture is not the expected BGRA8 target".to_owned());
    }
    // SAFETY: just checked: a 2D, single-mip, single-sample BGRA8 texture of
    // exactly `width × height`, created with a render-target binding.
    let hal_texture = unsafe {
        wgpu::hal::dx12::Device::texture_from_raw(
            resource,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureDimension::D2,
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            1,
            1,
        )
    };
    drop(hal_device);
    // SAFETY: the hal texture was opened on this device and matches the
    // descriptor; a freshly opened shared resource is in the common state.
    let texture = unsafe {
        device.create_texture_from_hal::<Dx12>(
            hal_texture,
            &descriptor(width, height),
            wgpu::TextureUses::UNINITIALIZED,
        )
    };
    Ok(texture)
}

#[cfg(not(any(target_os = "macos", windows)))]
pub(crate) fn wrap_ui_surface(
    _device: &wgpu::Device,
    _surface: u64,
    _width: u32,
    _height: u32,
) -> Result<wgpu::Texture, String> {
    Err("shared UI textures are not implemented on this platform".to_owned())
}

/// A device on the D3D12 adapter with this LUID, if there is one.
#[cfg(windows)]
#[allow(unsafe_code)]
fn gpu_on_adapter(luid: i64) -> Option<Gpu> {
    use wgpu::hal::api::Dx12;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::DX12));
    let adapter = adapters.into_iter().find(|adapter| {
        // SAFETY: we only query the raw DXGI adapter's description; the hal
        // guard does not outlive this closure.
        unsafe { adapter.as_hal::<Dx12>() }
            .and_then(|hal| unsafe { hal.raw_adapter().GetDesc1() }.ok())
            .is_some_and(|desc| {
                let id = desc.AdapterLuid;
                (i64::from(id.HighPart) << 32 | i64::from(id.LowPart)) == luid
            })
    })?;
    Gpu::from_adapter(&adapter).ok()
}

#[cfg(not(windows))]
fn gpu_on_adapter(_luid: i64) -> Option<Gpu> {
    None
}
