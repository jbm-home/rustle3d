#ifndef RUNNER_SHARED_TEXTURE_H_
#define RUNNER_SHARED_TEXTURE_H_

#include <d3d11.h>
#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>
#include <flutter/standard_method_codec.h>
#include <flutter/texture_registrar.h>
#include <wrl/client.h>

#include <cstdint>
#include <map>
#include <memory>

// GPU textures that the Rust core renders into and Flutter composites
// without any copy. Each one is a D3D11 texture created on Flutter's own
// adapter and shared through an NT handle: the Rust side opens the handle
// on its D3D12 device (crates/r3d-ffi/src/surface.rs), Flutter's ANGLE
// opens it on its D3D11 device.
//
// Method channel `dev.rustle3d/texture`, as on macOS:
// - `adapter` → the LUID of Flutter's adapter, for the Rust device;
// - `create` {width, height} → {textureId, surface (the NT handle)};
// - `resize` {textureId, width, height} → {textureId, surface}: here a new
//   texture replaces the old one;
// - `frame` {textureId, surface} → the texture has a new frame;
// - `dispose` {textureId}.
class SharedTextureBridge {
 public:
  explicit SharedTextureBridge(flutter::PluginRegistrarWindows* registrar);
  ~SharedTextureBridge();

  SharedTextureBridge(const SharedTextureBridge&) = delete;
  SharedTextureBridge& operator=(const SharedTextureBridge&) = delete;

 private:
  struct Shared;

  void Handle(const flutter::MethodCall<flutter::EncodableValue>& call,
              std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
                  result);

  // Unregister a texture, freed once Flutter no longer uses it.
  void Dispose(int64_t id);

  // The D3D11 device on Flutter's adapter, made on first use.
  bool EnsureDevice();
  std::unique_ptr<Shared> CreateShared(int width, int height);

  flutter::TextureRegistrar* textures_;
  Microsoft::WRL::ComPtr<IDXGIAdapter> adapter_;
  Microsoft::WRL::ComPtr<ID3D11Device> device_;
  std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>> channel_;
  std::map<int64_t, std::unique_ptr<Shared>> shared_;
};

#endif  // RUNNER_SHARED_TEXTURE_H_
