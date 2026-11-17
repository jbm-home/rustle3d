#include "shared_texture.h"

#include <dxgi1_2.h>

#include <utility>

using flutter::EncodableMap;
using flutter::EncodableValue;
using Microsoft::WRL::ComPtr;

// One shared texture: the D3D11 texture, its NT handle and what Flutter
// asks for to composite it.
struct SharedTextureBridge::Shared {
  ComPtr<ID3D11Texture2D> texture;
  HANDLE handle = nullptr;
  FlutterDesktopGpuSurfaceDescriptor descriptor = {};
  std::unique_ptr<flutter::TextureVariant> variant;

  ~Shared() {
    if (handle != nullptr) {
      CloseHandle(handle);
    }
  }
};

namespace {

// An integer argument of a method call, or `fallback`.
int64_t IntArg(const EncodableMap& args, const char* key, int64_t fallback) {
  auto it = args.find(EncodableValue(key));
  if (it == args.end()) {
    return fallback;
  }
  if (auto v = std::get_if<int32_t>(&it->second)) {
    return *v;
  }
  if (auto v = std::get_if<int64_t>(&it->second)) {
    return *v;
  }
  return fallback;
}

}  // namespace

SharedTextureBridge::SharedTextureBridge(
    flutter::PluginRegistrarWindows* registrar)
    : textures_(registrar->texture_registrar()) {
  registrar->GetGraphicsAdapter(adapter_.GetAddressOf());
  channel_ = std::make_unique<flutter::MethodChannel<EncodableValue>>(
      registrar->messenger(), "dev.rustle3d/texture",
      &flutter::StandardMethodCodec::GetInstance());
  channel_->SetMethodCallHandler([this](const auto& call, auto result) {
    Handle(call, std::move(result));
  });
}

SharedTextureBridge::~SharedTextureBridge() {
  channel_->SetMethodCallHandler(nullptr);
  for (auto& [id, shared] : shared_) {
    textures_->UnregisterTexture(id);
  }
}

bool SharedTextureBridge::EnsureDevice() {
  if (device_) {
    return true;
  }
  if (!adapter_) {
    return false;
  }
  // The adapter is given, so the driver type must be "unknown".
  HRESULT hr = D3D11CreateDevice(adapter_.Get(), D3D_DRIVER_TYPE_UNKNOWN,
                                 nullptr, 0, nullptr, 0, D3D11_SDK_VERSION,
                                 device_.GetAddressOf(), nullptr, nullptr);
  return SUCCEEDED(hr);
}

std::unique_ptr<SharedTextureBridge::Shared> SharedTextureBridge::CreateShared(
    int width, int height) {
  if (width <= 0 || height <= 0 || !EnsureDevice()) {
    return nullptr;
  }
  auto shared = std::make_unique<Shared>();
  D3D11_TEXTURE2D_DESC desc = {};
  desc.Width = static_cast<UINT>(width);
  desc.Height = static_cast<UINT>(height);
  desc.MipLevels = 1;
  desc.ArraySize = 1;
  desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
  desc.SampleDesc.Count = 1;
  desc.Usage = D3D11_USAGE_DEFAULT;
  desc.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
  // An NT handle, which D3D12 (the Rust side) can open.
  desc.MiscFlags =
      D3D11_RESOURCE_MISC_SHARED | D3D11_RESOURCE_MISC_SHARED_NTHANDLE;
  if (FAILED(device_->CreateTexture2D(&desc, nullptr,
                                      shared->texture.GetAddressOf()))) {
    return nullptr;
  }
  ComPtr<IDXGIResource1> resource;
  if (FAILED(shared->texture.As(&resource)) ||
      FAILED(resource->CreateSharedHandle(
          nullptr, DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE,
          nullptr, &shared->handle))) {
    return nullptr;
  }
  shared->descriptor.struct_size = sizeof(FlutterDesktopGpuSurfaceDescriptor);
  shared->descriptor.handle = shared->handle;
  shared->descriptor.width = shared->descriptor.visible_width = desc.Width;
  shared->descriptor.height = shared->descriptor.visible_height = desc.Height;
  shared->descriptor.format = kFlutterDesktopPixelFormatBGRA8888;
  shared->descriptor.release_callback = nullptr;
  shared->descriptor.release_context = nullptr;
  const FlutterDesktopGpuSurfaceDescriptor* descriptor = &shared->descriptor;
  shared->variant = std::make_unique<flutter::TextureVariant>(
      flutter::GpuSurfaceTexture(
          kFlutterDesktopGpuSurfaceTypeDxgiSharedHandle,
          [descriptor](size_t, size_t) { return descriptor; }));
  return shared;
}

void SharedTextureBridge::Handle(
    const flutter::MethodCall<EncodableValue>& call,
    std::unique_ptr<flutter::MethodResult<EncodableValue>> result) {
  const auto* args = std::get_if<EncodableMap>(call.arguments());
  const EncodableMap empty;
  const EncodableMap& a = args != nullptr ? *args : empty;
  const std::string& method = call.method_name();
  if (method == "adapter") {
    DXGI_ADAPTER_DESC desc = {};
    if (!adapter_ || FAILED(adapter_->GetDesc(&desc))) {
      result->Success();
      return;
    }
    const int64_t luid =
        (static_cast<int64_t>(desc.AdapterLuid.HighPart) << 32) |
        static_cast<int64_t>(desc.AdapterLuid.LowPart);
    result->Success(EncodableValue(luid));
  } else if (method == "create") {
    auto shared = CreateShared(static_cast<int>(IntArg(a, "width", 0)),
                               static_cast<int>(IntArg(a, "height", 0)));
    if (!shared) {
      result->Error("texture", "cannot create surface");
      return;
    }
    const int64_t id = textures_->RegisterTexture(shared->variant.get());
    const auto handle =
        static_cast<int64_t>(reinterpret_cast<intptr_t>(shared->handle));
    shared_[id] = std::move(shared);
    result->Success(EncodableValue(EncodableMap{
        {EncodableValue("textureId"), EncodableValue(id)},
        {EncodableValue("surface"), EncodableValue(handle)},
    }));
  } else if (method == "frame") {
    textures_->MarkTextureFrameAvailable(IntArg(a, "textureId", -1));
    result->Success();
  } else if (method == "dispose") {
    const int64_t id = IntArg(a, "textureId", -1);
    auto it = shared_.find(id);
    if (it != shared_.end()) {
      // Free the texture once Flutter no longer uses it.
      std::shared_ptr<Shared> gone = std::move(it->second);
      shared_.erase(it);
      textures_->UnregisterTexture(id, [gone]() {});
    }
    result->Success();
  } else {
    result->NotImplemented();
  }
}
