// A view rendered by the Rust core into a GPU texture shared with Flutter
// (no copy: macOS, Windows), or elsewhere into a texture of its own whose
// pixels are read back and shown as an image (Linux for now, or when the
// texture cannot be shared). The widget re-renders it every frame and
// forwards pointer input to the engine.

import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../design/tokens.dart';
import '../i18n.dart';
import '../rust/api/engine.dart';

const _channel = MethodChannel('dev.rustle3d/texture');

/// Tell the engine which GPU the UI composites on (Windows), so the views
/// render on it and share their textures. Call before the first view.
Future<void> prepareSharedTextures() async {
  if (!Platform.isWindows) return;
  try {
    final luid = await _channel.invokeMethod<int>('adapter');
    if (luid != null) setUiAdapter(luid: luid);
  } on PlatformException {
    // No adapter: the views read their pixels back.
  }
}

/// What a board tool (Polygon, Notch, Curve) does with the pointer, in
/// pattern coordinates (mm) with the board's millimetres per pixel.
class PatternPointer {
  const PatternPointer({this.onTap, this.onPress, this.onDrag, this.onRelease});

  /// A click (no drag).
  final void Function(Offset mm, double mmPerPx)? onTap;

  /// A press: true holds it (the next moves go to [onDrag], not to
  /// panning).
  final bool Function(Offset mm, double mmPerPx)? onPress;
  final ValueChanged<Offset>? onDrag;
  final VoidCallback? onRelease;
}

/// Paints over a board; `toScreen` places a pattern point (mm).
typedef BoardOverlay = void Function(
  Canvas canvas,
  Offset Function(Offset mm) toScreen,
);

class RustView extends StatefulWidget {
  const RustView({
    super.key,
    required this.engine,
    required this.kind,
    this.onSelection,
    this.onFrameTime,
    this.onPinned,
    this.drawing = false,
    this.onDrawn,
    this.holding = true,
    this.pattern,
    this.overlay,
  });

  final Engine engine;
  final ViewKind kind;

  /// Called with the new selection after a click.
  final ValueChanged<List<String>>? onSelection;

  /// Called with the Rust render time (CPU + GPU, ms) of each frame.
  final ValueChanged<double>? onFrameTime;

  /// Called with the new pin's id after an ⌥-click on the cloth.
  final ValueChanged<String>? onPinned;

  /// The Draw-on-garment tool is active: drags draw on the cloth.
  final bool drawing;

  /// Called with the new internal line's id after a stroke.
  final ValueChanged<String>? onDrawn;

  /// A press may hold what is under the pointer to drag it (on the board:
  /// a corner or an edge). Off, drags only pan or turn the view.
  final bool holding;

  /// A board tool's own pointer handling, instead of selecting and
  /// holding.
  final PatternPointer? pattern;

  /// Drawn over the board every frame.
  final BoardOverlay? overlay;

  @override
  State<RustView> createState() => _RustViewState();
}

class _RustViewState extends State<RustView>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker = createTicker((_) => _renderFrame());
  int? _textureId;
  int? _viewId;

  /// Shared textures (macOS, Windows); otherwise the pixels are copied.
  /// `--dart-define=R3D_COPY_PIXELS=true` tries the copied path there.
  /// Turned off for the session when a texture cannot be shared (e.g.
  /// the engine's GPU is not the one the UI composites on).
  static bool _shared =
      (Platform.isMacOS || Platform.isWindows) &&
      !const bool.fromEnvironment('R3D_COPY_PIXELS');

  /// The last frame read back (copied mode), and a read in flight.
  ui.Image? _image;
  bool _reading = false;

  /// A frame is rendering (shared mode).
  bool _rendering = false;
  Size _physical = Size.zero;
  double _ratio = 1;
  bool _creating = false;
  Offset _downAt = Offset.zero;
  bool _dragged = false;

  /// The pointer holds the cloth (3D view): moves pull it instead of
  /// orbiting.
  bool _holding = false;

  /// While lengthening a hem in 3D: the change so far (mm) and where to
  /// show it.
  PatternChange? _change;

  /// The stroke being drawn on the garment (screen points), if any.
  List<Offset>? _stroke;
  Offset _labelAt = Offset.zero;

  /// The trackpad pinch's scale at the last update.
  double _pinch = 1;

  /// A board tool holds the pointer.
  bool _patternHeld = false;

  /// Ticks every frame, to repaint the overlay.
  final _frames = ValueNotifier<int>(0);

  /// The pattern point (mm) under a local position of a board.
  Offset? _toPattern(Offset local) {
    final view = _viewId;
    if (view == null) return null;
    final p = widget.engine.toPattern(
      view: view,
      x: local.dx * _ratio,
      y: local.dy * _ratio,
    );
    return p == null ? null : Offset(p[0], p[1]);
  }

  /// Millimetres per logical pixel at `local`.
  double _mmPerPx(Offset local) {
    final (a, b) = (_toPattern(local), _toPattern(local + const Offset(1, 0)));
    return a == null || b == null ? 1 : (b - a).distance;
  }

  Offset _toScreen(Offset mm) {
    final view = _viewId;
    if (view == null) return Offset.zero;
    final p = widget.engine.toView(view: view, xMm: mm.dx, yMm: mm.dy);
    return p == null ? Offset.zero : Offset(p[0], p[1]) / _ratio;
  }

  /// Zoom by `amount` (positive = in) about the point under the pointer.
  void _zoom(Offset at, double amount) {
    final view = _viewId;
    if (view == null || amount == 0 || !amount.isFinite) return;
    widget.engine.zoom(
      view: view,
      x: at.dx * _ratio,
      y: at.dy * _ratio,
      amount: amount,
    );
  }

  @override
  void initState() {
    super.initState();
    _ticker.start();
  }

  @override
  void dispose() {
    _ticker.dispose();
    _frames.dispose();
    _release();
    _image?.dispose();
    super.dispose();
  }

  void _release() {
    final view = _viewId;
    final texture = _textureId;
    _viewId = null;
    _textureId = null;
    if (view != null) {
      widget.engine.detachView(view: view);
    }
    if (texture != null) {
      _channel.invokeMethod('dispose', {'textureId': texture});
    }
  }

  Future<void> _resize(Size physical) async {
    if (_creating || physical == _physical || physical.isEmpty) return;
    _creating = true;
    _physical = physical;
    _release();
    if (!_shared) {
      _viewId = widget.engine.attachView(
        kind: widget.kind,
        surface: null,
        width: physical.width.round(),
        height: physical.height.round(),
      );
      _creating = false;
      return;
    }
    final created = await _channel.invokeMapMethod<String, Object>('create', {
      'width': physical.width.round(),
      'height': physical.height.round(),
    });
    final textureId = created?['textureId'] as int?;
    final surface = created?['surface'] as int?;
    if (mounted && textureId != null && surface != null) {
      try {
        _viewId = widget.engine.attachView(
          kind: widget.kind,
          surface: BigInt.from(surface),
          width: physical.width.round(),
          height: physical.height.round(),
        );
        setState(() => _textureId = textureId);
      } on ApiError catch (e) {
        if (e.code != 'ui_surface') rethrow;
        // Read the pixels back instead, from now on.
        _channel.invokeMethod('dispose', {'textureId': textureId});
        _shared = false;
        _physical = Size.zero;
        _creating = false;
        if (mounted) setState(() {});
        return;
      }
    }
    _creating = false;
  }

  void _renderFrame() {
    final view = _viewId;
    if (view == null) return;
    if (widget.overlay != null) _frames.value++;
    if (!_shared) {
      if (!_reading) _readBack(view);
      return;
    }
    final texture = _textureId;
    if (texture == null || _rendering) return;
    _render(view, texture);
  }

  /// Shared mode: render off the UI thread, then hand the frame to the
  /// compositor; one frame at a time.
  Future<void> _render(int view, int texture) async {
    _rendering = true;
    try {
      final ms = await widget.engine.renderView(view: view);
      if (!mounted || view != _viewId) return;
      _channel.invokeMethod('frame', {'textureId': texture});
      widget.onFrameTime?.call(ms);
    } on ApiError {
      // The view went away while it rendered.
    } finally {
      _rendering = false;
    }
  }

  /// Copied mode: render, read the pixels back and show them; one frame
  /// at a time.
  Future<void> _readBack(int view) async {
    _reading = true;
    final double ms;
    try {
      ms = await widget.engine.renderView(view: view);
    } on ApiError {
      _reading = false;
      return;
    }
    if (!mounted || view != _viewId) {
      _reading = false;
      return;
    }
    widget.onFrameTime?.call(ms);
    final size = _physical;
    final pixels = await widget.engine.readView(view: view);
    ui.decodeImageFromPixels(
      pixels,
      size.width.round(),
      size.height.round(),
      ui.PixelFormat.bgra8888,
      (image) {
        _reading = false;
        if (!mounted || view != _viewId) {
          image.dispose();
          return;
        }
        setState(() {
          _image?.dispose();
          _image = image;
        });
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    // Copied pixels cost bandwidth: render those at one pixel per point.
    _ratio = _shared ? MediaQuery.devicePixelRatioOf(context) : 1;
    return LayoutBuilder(
      builder: (context, constraints) {
        final physical = Size(
          (constraints.maxWidth * _ratio).floorToDouble(),
          (constraints.maxHeight * _ratio).floorToDouble(),
        );
        if (physical != _physical) {
          WidgetsBinding.instance.addPostFrameCallback(
            (_) => _resize(physical),
          );
        }
        final texture = _textureId;
        return Listener(
          onPointerDown: (e) {
            _downAt = e.localPosition;
            _dragged = false;
            final press = widget.pattern?.onPress;
            if (press != null) {
              final mm = _toPattern(e.localPosition);
              _patternHeld = mm != null && press(mm, _mmPerPx(e.localPosition));
              if (_patternHeld) return;
            }
            final view = _viewId;
            // ⌥-click pins the cloth under the cursor.
            if (view != null && widget.drawing) {
              final started = widget.engine.strokeStart(
                view: view,
                x: e.localPosition.dx * _ratio,
                y: e.localPosition.dy * _ratio,
              );
              if (started) setState(() => _stroke = [e.localPosition]);
              return;
            }
            if (view != null && HardwareKeyboard.instance.isAltPressed) {
              final pin = widget.engine.pinAt(
                view: view,
                x: e.localPosition.dx * _ratio,
                y: e.localPosition.dy * _ratio,
              );
              if (pin != null) {
                _dragged = true; // not a selection click
                widget.onPinned?.call(pin);
                return;
              }
            }
            _holding =
                view != null &&
                widget.holding &&
                widget.pattern == null &&
                widget.engine.press(
                  view: view,
                  x: e.localPosition.dx * _ratio,
                  y: e.localPosition.dy * _ratio,
                  // ⇧-drag on a hem lengthens the garment; ⌘-drag (Ctrl
                  // elsewhere) moves the piece around the body.
                  gesture: switch (HardwareKeyboard.instance) {
                    final k when k.isMetaPressed || k.isControlPressed =>
                      Gesture.move,
                    final k when k.isShiftPressed => Gesture.adjust,
                    _ => Gesture.pull,
                  },
                );
          },
          onPointerMove: (e) {
            final view = _viewId;
            if (view == null) return;
            if ((e.localPosition - _downAt).distance > 3) _dragged = true;
            if (_patternHeld) {
              if (_toPattern(e.localPosition) case final mm?) {
                widget.pattern?.onDrag?.call(mm);
              }
              return;
            }
            if (_stroke case final stroke?) {
              widget.engine.strokeTo(
                view: view,
                x: e.localPosition.dx * _ratio,
                y: e.localPosition.dy * _ratio,
              );
              setState(() => stroke.add(e.localPosition));
              return;
            }
            if (widget.drawing) return;
            if (_holding) {
              final change = widget.engine.pull(
                view: view,
                x: e.localPosition.dx * _ratio,
                y: e.localPosition.dy * _ratio,
              );
              if (change != null) {
                setState(() {
                  _change = change;
                  _labelAt = e.localPosition;
                });
              }
              return;
            }
            widget.engine.drag(
              view: view,
              dx: e.delta.dx * _ratio,
              dy: e.delta.dy * _ratio,
            );
          },
          onPointerUp: (e) {
            if (_patternHeld) {
              _patternHeld = false;
              widget.pattern?.onRelease?.call();
              return;
            }
            final tap = widget.pattern?.onTap;
            if (tap != null) {
              final mm = _dragged ? null : _toPattern(e.localPosition);
              if (mm != null) tap(mm, _mmPerPx(e.localPosition));
              return;
            }
            if (_stroke != null) {
              setState(() => _stroke = null);
              try {
                final line = widget.engine.strokeEnd();
                if (line != null) widget.onDrawn?.call(line);
              } on ApiError catch (error) {
                ScaffoldMessenger.maybeOf(context)?.showSnackBar(
                  SnackBar(
                    content: Text(context.error(error)),
                    behavior: SnackBarBehavior.floating,
                  ),
                );
              }
              return;
            }
            if (widget.drawing) return;
            if (_holding) {
              _holding = false;
              widget.engine.release();
              if (_change != null) {
                setState(() => _change = null);
                // The pattern changed: let the shell refresh.
                widget.onSelection?.call(widget.engine.selection());
              }
            }
            final view = _viewId;
            if (view == null || _dragged) return;
            final selection = widget.engine.click(
              view: view,
              x: e.localPosition.dx * _ratio,
              y: e.localPosition.dy * _ratio,
              // ⇧-click a second corner to measure between them.
              extend: HardwareKeyboard.instance.isShiftPressed,
            );
            widget.onSelection?.call(selection);
          },
          onPointerSignal: (e) {
            if (e is PointerScrollEvent) {
              _zoom(e.localPosition, -e.scrollDelta.dy / 120);
            }
          },
          // Trackpads send gestures, not scrolls: a pinch zooms by its
          // scale, two fingers moving up or down zoom like a wheel.
          onPointerPanZoomStart: (_) => _pinch = 1,
          onPointerPanZoomUpdate: (e) {
            final ratio = e.scale / _pinch;
            _pinch = e.scale;
            // The engine scales by (1 − amount / 10).
            _zoom(e.localPosition, 10 * (1 - 1 / ratio) + e.panDelta.dy / 120);
          },
          child: Stack(
            children: [
              Positioned.fill(
                child: AnimatedOpacity(
                  opacity: texture == null && _image == null ? 0 : 1,
                  duration: const Duration(milliseconds: 220),
                  child: switch ((texture, _image)) {
                    (final int id, _) => Texture(textureId: id),
                    (_, final ui.Image image) => RawImage(
                      image: image,
                      fit: BoxFit.fill,
                    ),
                    _ => const SizedBox.expand(),
                  },
                ),
              ),
              if (_stroke case final stroke?)
                Positioned.fill(
                  child: IgnorePointer(
                    child: CustomPaint(
                      painter: _StrokePainter(
                        stroke,
                        Tokens.of(context).accent,
                      ),
                    ),
                  ),
                ),
              if (widget.overlay case final overlay?)
                Positioned.fill(
                  child: IgnorePointer(
                    child: CustomPaint(
                      painter: _OverlayPainter(overlay, _toScreen, _frames),
                    ),
                  ),
                ),
              if (_change case final change?)
                Positioned(
                  left: _labelAt.dx + 14,
                  top: _labelAt.dy - 30,
                  child: _ChangeLabel(change: change),
                ),
            ],
          ),
        );
      },
    );
  }
}

/// "Longer 30 mm", "Taken in 12 mm", "Moved 8 mm"… next to the cursor while changing
/// the pattern in 3D.
class _ChangeLabel extends StatelessWidget {
  const _ChangeLabel({required this.change});

  final PatternChange change;

  String _text(BuildContext context) {
    final l = context.l;
    final mm = change.mm;
    final amount = context.number(mm.abs());
    return switch (change.kind) {
      'fit' => mm >= 0 ? l.changeTakenIn(amount) : l.changeLetOut(amount),
      'move' => l.changeMoved(amount),
      _ => mm >= 0 ? l.changeLonger(amount) : l.changeShorter(amount),
    };
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return IgnorePointer(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
        decoration: BoxDecoration(
          color: p.text.withValues(alpha: 0.85),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Text(
          _text(context),
          style: ts(
            fontSize: 12.5,
            fontWeight: FontWeight.w600,
            color: p.surface,
          ),
        ),
      ),
    );
  }
}

/// The line being drawn on the garment, under the cursor.
class _StrokePainter extends CustomPainter {
  _StrokePainter(this.points, this.color);

  final List<Offset> points;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (points.length < 2) return;
    final path = Path()..moveTo(points.first.dx, points.first.dy);
    for (final p in points.skip(1)) {
      path.lineTo(p.dx, p.dy);
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = color
        ..strokeWidth = 3
        ..style = PaintingStyle.stroke
        ..strokeCap = StrokeCap.round
        ..strokeJoin = StrokeJoin.round,
    );
  }

  @override
  bool shouldRepaint(_StrokePainter old) => true;
}

/// A board overlay, repainted every frame (the board may pan or zoom).
class _OverlayPainter extends CustomPainter {
  _OverlayPainter(this.overlay, this.toScreen, Listenable frames)
    : super(repaint: frames);

  final BoardOverlay overlay;
  final Offset Function(Offset mm) toScreen;

  @override
  void paint(Canvas canvas, Size size) => overlay(canvas, toScreen);

  @override
  bool shouldRepaint(_OverlayPainter old) => true;
}
