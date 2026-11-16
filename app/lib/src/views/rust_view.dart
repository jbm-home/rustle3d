// A view rendered by the Rust core into a GPU texture shared with Flutter
// (no copy, macOS), or elsewhere into a texture of its own whose pixels are
// read back and shown as an image (Linux, Windows: shared textures come
// later). The widget re-renders it every frame and forwards pointer input
// to the engine.

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

  @override
  State<RustView> createState() => _RustViewState();
}

class _RustViewState extends State<RustView>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker = createTicker((_) => _renderFrame());
  int? _textureId;
  int? _viewId;

  /// Shared textures (macOS); otherwise the pixels are copied.
  /// `--dart-define=R3D_COPY_PIXELS=true` tries the copied path on macOS.
  static final bool _shared =
      Platform.isMacOS && !const bool.fromEnvironment('R3D_COPY_PIXELS');

  /// The last frame read back (copied mode), and a read in flight.
  ui.Image? _image;
  bool _reading = false;
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

  @override
  void initState() {
    super.initState();
    _ticker.start();
  }

  @override
  void dispose() {
    _ticker.dispose();
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
      _viewId = widget.engine.attachView(
        kind: widget.kind,
        surface: BigInt.from(surface),
        width: physical.width.round(),
        height: physical.height.round(),
      );
      setState(() => _textureId = textureId);
    }
    _creating = false;
  }

  void _renderFrame() {
    final view = _viewId;
    if (view == null) return;
    if (!_shared) {
      if (!_reading) _readBack(view);
      return;
    }
    final texture = _textureId;
    if (texture == null) return;
    final ms = widget.engine.renderView(view: view);
    _channel.invokeMethod('frame', {'textureId': texture});
    widget.onFrameTime?.call(ms);
  }

  /// Copied mode: render, read the pixels back and show them; one frame
  /// at a time.
  Future<void> _readBack(int view) async {
    _reading = true;
    final ms = widget.engine.renderView(view: view);
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
            );
            widget.onSelection?.call(selection);
          },
          onPointerSignal: (e) {
            final view = _viewId;
            if (view == null || e is! PointerScrollEvent) return;
            widget.engine.zoom(
              view: view,
              x: e.localPosition.dx * _ratio,
              y: e.localPosition.dy * _ratio,
              amount: -e.scrollDelta.dy / 120,
            );
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

/// "Longer 30 mm", "Taken in 12 mm"… next to the cursor while changing
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
