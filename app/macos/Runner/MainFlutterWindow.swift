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


/// An IOSurface that the Rust core renders into and Flutter composites
/// without any copy: the memory is wrapped as a Metal texture on the Rust
/// side (crates/r3d-ffi/src/surface.rs) and as a CVPixelBuffer here.
struct SharedSurface {
  let surface: IOSurface
  let pixelBuffer: CVPixelBuffer

  init?(width: Int, height: Int) {
    guard width > 0, height > 0 else { return nil }
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
  var pointer: Int {
    Int(bitPattern: Unmanaged.passUnretained(surface).toOpaque())
  }
}

/// One view's texture, registered with Flutter once for the life of the
/// view. A resize swaps the surface behind it instead of registering a new
/// texture: Flutter's macOS texture registry is not safe to change while
/// the raster thread composites a frame that uses one of its textures
/// (unregistering during a window resize crashed the app).
final class SharedTexture: NSObject, FlutterTexture {
  /// Guards `shown`, read by Flutter's raster thread.
  private let lock = NSLock()
  /// The surface Flutter composites.
  private var shown: SharedSurface
  /// The surface of the last resize, shown from its first frame on.
  private var next: SharedSurface?

  init(surface: SharedSurface) {
    shown = surface
  }

  /// Make a surface of the new size; it replaces the shown one at its
  /// first frame, so the old picture stays up until then.
  func resize(width: Int, height: Int) -> SharedSurface? {
    next = SharedSurface(width: width, height: height)
    return next
  }

  /// The surface at `pointer` has a new frame. False when it is neither
  /// the shown surface nor the next one (a frame of a replaced surface).
  func frame(surface pointer: Int) -> Bool {
    if let next, next.pointer == pointer {
      lock.lock()
      shown = next
      lock.unlock()
      self.next = nil
      return true
    }
    lock.lock()
    defer { lock.unlock() }
    return shown.pointer == pointer
  }

  func copyPixelBuffer() -> Unmanaged<CVPixelBuffer>? {
    lock.lock()
    defer { lock.unlock() }
    return Unmanaged.passRetained(shown.pixelBuffer)
  }
}

/// Method channel `dev.rustle3d/texture`: create, resize, refresh and
/// dispose shared textures for the Dart `RustView` widget.
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
    let id = args["textureId"] as? Int64
    let width = args["width"] as? Int ?? 0
    let height = args["height"] as? Int ?? 0
    switch call.method {
    case "create":
      guard let surface = SharedSurface(width: width, height: height) else {
        result(FlutterError(code: "texture", message: "cannot create surface", details: nil))
        return
      }
      let texture = SharedTexture(surface: surface)
      let id = registry.register(texture)
      textures[id] = texture
      result(["textureId": id, "surface": surface.pointer])
    case "resize":
      guard let id, let surface = textures[id]?.resize(width: width, height: height) else {
        result(FlutterError(code: "texture", message: "cannot create surface", details: nil))
        return
      }
      result(["textureId": id, "surface": surface.pointer])
    case "frame":
      if let id, let surface = args["surface"] as? Int,
        textures[id]?.frame(surface: surface) == true
      {
        registry.textureFrameAvailable(id)
      }
      result(nil)
    case "dispose":
      if let id, textures.removeValue(forKey: id) != nil {
        registry.unregisterTexture(id)
      }
      result(nil)
    default:
      result(FlutterMethodNotImplemented)
    }
  }
}
