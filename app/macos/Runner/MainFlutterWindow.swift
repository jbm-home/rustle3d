import Cocoa
import CoreVideo
import FlutterMacOS
import IOSurface

class MainFlutterWindow: NSWindow {
  private var sharedTextures: SharedTextureBridge?

  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    self.contentViewController = flutterViewController
    // Open large and centred: the pattern and the 3D view need room.
    if let screen = NSScreen.main?.visibleFrame {
      let size = NSSize(
        width: min(1600, screen.width * 0.9), height: min(1000, screen.height * 0.9))
      let origin = NSPoint(
        x: screen.midX - size.width / 2, y: screen.midY - size.height / 2)
      self.setFrame(NSRect(origin: origin, size: size), display: true)
    }
    self.titlebarAppearsTransparent = true
    self.titleVisibility = .hidden
    self.styleMask.insert(.fullSizeContentView)
    self.minSize = NSSize(width: 960, height: 640)
    // Always open at the size above, not at the last session's frame.
    self.isRestorable = false

    #if DEBUG
      // Lets tooling capture just this window (`screencapture -l <id>`)
      // instead of the whole screen.
      DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak self] in
        guard let number = self?.windowNumber else { return }
        let file = URL(fileURLWithPath: NSTemporaryDirectory())
          .appendingPathComponent("rustle3d-window-id")
        try? String(number).write(to: file, atomically: true, encoding: .utf8)
      }
    #endif

    RegisterGeneratedPlugins(registry: flutterViewController)
    sharedTextures = SharedTextureBridge(
      registrar: flutterViewController.registrar(forPlugin: "SharedTextures"))

    super.awakeFromNib()
  }
}


/// A GPU surface that the Rust core renders into and Flutter composites
/// without any copy: the IOSurface memory is wrapped as a Metal texture on
/// the Rust side (crates/r3d-ffi/src/surface.rs) and as a CVPixelBuffer here.
final class SharedTexture: NSObject, FlutterTexture {
  let surface: IOSurface
  private let pixelBuffer: CVPixelBuffer

  init?(width: Int, height: Int) {
    let properties: [IOSurfacePropertyKey: Any] = [
      .width: width,
      .height: height,
      .bytesPerElement: 4,
      .pixelFormat: kCVPixelFormatType_32BGRA,
    ]
    guard let surface = IOSurface(properties: properties) else { return nil }
    var buffer: Unmanaged<CVPixelBuffer>?
    let attributes = [kCVPixelBufferMetalCompatibilityKey: true] as CFDictionary
    let status = CVPixelBufferCreateWithIOSurface(
      kCFAllocatorDefault, surface, attributes, &buffer)
    guard status == kCVReturnSuccess, let buffer else { return nil }
    self.surface = surface
    self.pixelBuffer = buffer.takeRetainedValue()
  }

  /// Pointer to the IOSurfaceRef, handed to Rust (same process).
  var surfacePointer: Int {
    Int(bitPattern: Unmanaged.passUnretained(surface).toOpaque())
  }

  func copyPixelBuffer() -> Unmanaged<CVPixelBuffer>? {
    Unmanaged.passRetained(pixelBuffer)
  }
}

/// Method channel `dev.rustle3d/texture`: create, refresh and dispose shared
/// textures for the Dart `RustView` widget.
final class SharedTextureBridge {
  private let registry: FlutterTextureRegistry
  private let channel: FlutterMethodChannel
  private var textures: [Int64: SharedTexture] = [:]

  init(registrar: FlutterPluginRegistrar) {
    registry = registrar.textures
    channel = FlutterMethodChannel(
      name: "dev.rustle3d/texture", binaryMessenger: registrar.messenger)
    channel.setMethodCallHandler { [weak self] call, result in
      self?.handle(call, result: result)
    }
  }

  private func handle(_ call: FlutterMethodCall, result: FlutterResult) {
    let args = call.arguments as? [String: Any] ?? [:]
    switch call.method {
    case "create":
      guard let width = args["width"] as? Int, let height = args["height"] as? Int,
        width > 0, height > 0, let texture = SharedTexture(width: width, height: height)
      else {
        result(FlutterError(code: "texture", message: "cannot create surface", details: nil))
        return
      }
      let id = registry.register(texture)
      textures[id] = texture
      result(["textureId": id, "surface": texture.surfacePointer])
    case "frame":
      if let id = args["textureId"] as? Int64 {
        registry.textureFrameAvailable(id)
      }
      result(nil)
    case "dispose":
      if let id = args["textureId"] as? Int64 {
        registry.unregisterTexture(id)
        textures.removeValue(forKey: id)
      }
      result(nil)
    default:
      result(FlutterMethodNotImplemented)
    }
  }
}
