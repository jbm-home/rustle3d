// A view rendered by the Rust core into a GPU texture shared with Flutter
// (no copy). The widget owns the texture, re-renders it every frame and
// forwards pointer input to the engine.

import 'package:flutter/gestures.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../rust/api/engine.dart';

const _channel = MethodChannel('dev.rustle3d/texture');

class RustView extends StatefulWidget {
  const RustView({
    super.key,
    required this.engine,
    required this.kind,
    this.onSelection,
    this.onFrameTime,
  });

  final Engine engine;
  final ViewKind kind;

  /// Called with the new selection after a click.
  final ValueChanged<List<String>>? onSelection;

  /// Called with the Rust render time (CPU + GPU, ms) of each frame.
  final ValueChanged<double>? onFrameTime;

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
          },
          onPointerMove: (e) {
            final view = _viewId;
            if (view == null) return;
            if ((e.localPosition - _downAt).distance > 3) _dragged = true;
            widget.engine.drag(
              view: view,
              dx: e.delta.dx * _ratio,
              dy: e.delta.dy * _ratio,
            );
          },
          onPointerUp: (e) {
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
          child: AnimatedOpacity(
            opacity: texture == null ? 0 : 1,
            duration: const Duration(milliseconds: 220),
            child: texture == null
                ? const SizedBox.expand()
                : Texture(textureId: texture),
          ),
        );
      },
    );
  }
}
