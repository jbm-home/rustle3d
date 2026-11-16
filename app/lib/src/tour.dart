// The first-launch tour (docs/ux.md §1): a few coach marks over the main
// window, each pointing at one area. Shown once; it can be skipped.

import 'dart:io';

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';

/// One stop of the tour: the area it points at and what it says.
class TourStop {
  const TourStop(this.target, this.title, this.text);

  final GlobalKey target;
  final String title;
  final String text;
}

/// Whether the tour was already seen, remembered in the user's library
/// folder.
class TourMemory {
  const TourMemory(this.libraryDir);

  final String libraryDir;

  File get _file => File('$libraryDir/tour-seen');

  bool get seen => _file.existsSync();

  void markSeen() {
    try {
      _file.createSync(recursive: true);
    } on FileSystemException {
      // Not remembered: the tour shows again next time, nothing worse.
    }
  }
}

/// [text] with `**…**` runs in bold, in [strong] (keys, button names).
TextSpan marked(String text, TextStyle style, Color strong) {
  final parts = text.split('**');
  return TextSpan(
    style: style,
    children: [
      for (var i = 0; i < parts.length; i++)
        TextSpan(
          text: parts[i],
          style: i.isOdd
              ? TextStyle(fontWeight: FontWeight.w700, color: strong)
              : null,
        ),
    ],
  );
}

/// Show [stops] one after the other over the window; the last stop's
/// button says [done] (default: Start). Stop texts may mark keys in
/// `**bold**`.
Future<void> showTour(
  BuildContext context,
  List<TourStop> stops, {
  String? done,
}) {
  return Navigator.of(context).push(
    PageRouteBuilder<void>(
      opaque: false,
      barrierDismissible: false,
      transitionDuration: Motion.normal,
      pageBuilder: (_, _, _) => _Tour(stops: stops, done: done),
      transitionsBuilder: (_, animation, _, child) =>
          FadeTransition(opacity: animation, child: child),
    ),
  );
}

class _Tour extends StatefulWidget {
  const _Tour({required this.stops, required this.done});

  final List<TourStop> stops;
  final String? done;

  @override
  State<_Tour> createState() => _TourState();
}

class _TourState extends State<_Tour> {
  int _at = 0;

  Rect? _rectOf(GlobalKey key) {
    final box = key.currentContext?.findRenderObject() as RenderBox?;
    if (box == null || !box.hasSize) return null;
    return box.localToGlobal(Offset.zero) & box.size;
  }

  void _next() {
    if (_at + 1 < widget.stops.length) {
      setState(() => _at++);
    } else {
      Navigator.pop(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final stop = widget.stops[_at];
    final screen = MediaQuery.sizeOf(context);
    final target = (_rectOf(stop.target) ?? Rect.zero).inflate(6);
    // The card goes below the target if there is room, otherwise above,
    // otherwise inside it (a tall view); horizontally it stays on screen.
    const cardWidth = 320.0;
    const cardHeight = 190.0;
    final below = target.bottom + cardHeight < screen.height;
    final above = target.top - cardHeight > 0;
    final double? top = below
        ? target.bottom + Space.m
        : (above ? null : target.center.dy - cardHeight / 2);
    final double? bottom = !below && above
        ? screen.height - target.top + Space.m
        : null;
    final left = (target.center.dx - cardWidth / 2).clamp(
      Space.l,
      screen.width - cardWidth - Space.l,
    );
    return Stack(
      children: [
        Positioned.fill(
          child: GestureDetector(
            onTap: _next,
            child: CustomPaint(
              painter: _Dim(target, Colors.black.withValues(alpha: 0.45)),
            ),
          ),
        ),
        AnimatedPositioned(
          duration: Motion.normal,
          curve: Motion.curve,
          left: left,
          top: top,
          bottom: bottom,
          width: cardWidth,
          child: Material(
            color: p.surface,
            elevation: 8,
            borderRadius: BorderRadius.circular(kRadius),
            child: Padding(
              padding: const EdgeInsets.all(Space.l),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    stop.title,
                    style: ts(
                      fontSize: 15,
                      fontWeight: FontWeight.w700,
                      color: p.text,
                    ),
                  ),
                  const SizedBox(height: 6),
                  Text.rich(
                    marked(
                      stop.text,
                      ts(fontSize: 13, color: p.textMuted, height: 1.4),
                      p.text,
                    ),
                  ),
                  const SizedBox(height: Space.m),
                  Row(
                    children: [
                      Text(
                        '${_at + 1} / ${widget.stops.length}',
                        style: ts(fontSize: 12, color: p.textMuted),
                      ),
                      const Spacer(),
                      if (_at + 1 < widget.stops.length)
                        TextButton(
                          onPressed: () => Navigator.pop(context),
                          child: Text(context.l.skip),
                        ),
                      const SizedBox(width: Space.xs),
                      FilledButton(
                        onPressed: _next,
                        child: Text(
                          _at + 1 < widget.stops.length
                              ? context.l.next
                              : widget.done ?? context.l.start,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}

/// Dims everything but a rounded hole around the target.
class _Dim extends CustomPainter {
  const _Dim(this.hole, this.color);

  final Rect hole;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final path = Path()
      ..fillType = PathFillType.evenOdd
      ..addRect(Offset.zero & size)
      ..addRRect(RRect.fromRectAndRadius(hole, const Radius.circular(kRadius)));
    canvas.drawPath(path, Paint()..color = color);
  }

  @override
  bool shouldRepaint(_Dim old) => old.hole != hole || old.color != color;
}
