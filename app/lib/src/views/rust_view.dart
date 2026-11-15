// A view rendered by the Rust core into a GPU texture shared with Flutter
// (no copy). The widget owns the texture, re-renders it every frame and
// forwards pointer input to the engine.

import 'package:flutter/gestures.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../design/tokens.dart';
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
  });

  final Engine engine;
  final ViewKind kind;

  /// Called with the new selection after a click.
  final ValueChanged<List<String>>? onSelection;

  /// Called with the Rust render time (CPU + GPU, ms) of each frame.
  final ValueChanged<double>? onFrameTime;

  /// Called with the new pin's id after an ⌥-click on the cloth.
  final ValueChanged<String>? onPinned;

  @override
  State<RustView> createState() => _RustViewState();
}

class _RustViewState extends State<RustView>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker = createTicker((_) => _renderFrame());
  int? _textureId;
  int? _viewId;
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
  double? _lengthMm;
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
    final texture = _textureId;
    if (view == null || texture == null) return;
    final ms = widget.engine.renderView(view: view);
    _channel.invokeMethod('frame', {'textureId': texture});
    widget.onFrameTime?.call(ms);
  }

  @override
  Widget build(BuildContext context) {
    _ratio = MediaQuery.devicePixelRatioOf(context);
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
                  // ⇧-drag on a hem lengthens the garment.
                  lengthen: HardwareKeyboard.instance.isShiftPressed,
                );
          },
          onPointerMove: (e) {
            final view = _viewId;
            if (view == null) return;
            if ((e.localPosition - _downAt).distance > 3) _dragged = true;
            if (_holding) {
              final mm = widget.engine.pull(
                view: view,
                x: e.localPosition.dx * _ratio,
                y: e.localPosition.dy * _ratio,
              );
              if (mm != null) {
                setState(() {
                  _lengthMm = mm;
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
            if (_holding) {
              _holding = false;
              widget.engine.release();
              if (_lengthMm != null) {
                setState(() => _lengthMm = null);
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
                  opacity: texture == null ? 0 : 1,
                  duration: const Duration(milliseconds: 220),
                  child: texture == null
                      ? const SizedBox.expand()
                      : Texture(textureId: texture),
                ),
              ),
              if (_lengthMm case final mm?)
                Positioned(
                  left: _labelAt.dx + 14,
                  top: _labelAt.dy - 30,
                  child: _LengthLabel(mm: mm),
                ),
            ],
          ),
        );
      },
    );
  }
}

/// "+30 mm" next to the cursor while lengthening in 3D.
class _LengthLabel extends StatelessWidget {
  const _LengthLabel({required this.mm});

  final double mm;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final sign = mm > 0 ? '+' : (mm < 0 ? '−' : '');
    return IgnorePointer(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
        decoration: BoxDecoration(
          color: p.text.withValues(alpha: 0.85),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Text(
          '${mm > 0 ? 'Longer' : 'Shorter'} $sign${mm.abs().toStringAsFixed(0)} mm',
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
