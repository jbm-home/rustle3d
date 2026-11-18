// The projector view (docs/specs/23-production.md): the pattern at 1:1 on
// a projector hung over the cutting table, in high contrast, mirrored or
// turned if the room needs it, moved to bring a part of it under the
// lamp. Its calibration belongs to the room and the projector, so it is
// an app preference (projector.json in the user's library folder), never
// part of a project.

import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';

/// Logical pixels per millimetre before any calibration: 96 per inch.
const defaultPxPerMm = 96 / 25.4;

/// Side of the calibration square (mm).
const calibrationMm = 200.0;

/// Length of the scale bar always shown in a corner (mm).
const _scaleBarMm = 100.0;

/// How the projector is set up.
class ProjectorSettings {
  const ProjectorSettings({
    this.pxPerMmX = defaultPxPerMm,
    this.pxPerMmY = defaultPxPerMm,
    this.mirror = false,
    this.quarterTurns = 0,
    this.dark = true,
  });

  /// Logical pixels per millimetre on the table, across and down.
  final double pxPerMmX;
  final double pxPerMmY;

  /// Turned over left to right.
  final bool mirror;

  /// Turned by this many quarter turns, anticlockwise.
  final int quarterTurns;

  /// White lines on black (true) or black on white.
  final bool dark;

  ProjectorSettings copyWith({
    double? pxPerMmX,
    double? pxPerMmY,
    bool? mirror,
    int? quarterTurns,
    bool? dark,
  }) => ProjectorSettings(
    pxPerMmX: pxPerMmX ?? this.pxPerMmX,
    pxPerMmY: pxPerMmY ?? this.pxPerMmY,
    mirror: mirror ?? this.mirror,
    quarterTurns: (quarterTurns ?? this.quarterTurns) % 4,
    dark: dark ?? this.dark,
  );

  /// The scale after the calibration square ([calibrationMm] at this
  /// scale) was measured [widthMm] × [heightMm] on the table.
  ProjectorSettings calibrated(double widthMm, double heightMm) => copyWith(
    pxPerMmX: pxPerMmX * calibrationMm / widthMm,
    pxPerMmY: pxPerMmY * calibrationMm / heightMm,
  );

  Map<String, Object?> toJson() => {
    'px_per_mm_x': pxPerMmX,
    'px_per_mm_y': pxPerMmY,
    'mirror': mirror,
    'quarter_turns': quarterTurns,
    'dark': dark,
  };

  static ProjectorSettings fromJson(Map<String, Object?> json) {
    double scale(Object? v) => v is num && v.isFinite && v > 0.1 && v < 100
        ? v.toDouble()
        : defaultPxPerMm;
    return ProjectorSettings(
      pxPerMmX: scale(json['px_per_mm_x']),
      pxPerMmY: scale(json['px_per_mm_y']),
      mirror: json['mirror'] == true,
      quarterTurns: switch (json['quarter_turns']) {
        final int turns => turns % 4,
        _ => 0,
      },
      dark: json['dark'] != false,
    );
  }
}

/// The projector's settings, remembered in the user's library folder.
class ProjectorMemory {
  ProjectorMemory(this.libraryDir);

  final String libraryDir;

  File get _file => File('$libraryDir/projector.json');

  ProjectorSettings read() {
    try {
      final json = jsonDecode(_file.readAsStringSync());
      return json is Map<String, Object?>
          ? ProjectorSettings.fromJson(json)
          : const ProjectorSettings();
    } on FileSystemException {
      return const ProjectorSettings();
    } on FormatException {
      return const ProjectorSettings();
    }
  }

  void write(ProjectorSettings settings) {
    try {
      _file
        ..createSync(recursive: true)
        ..writeAsStringSync(jsonEncode(settings.toJson()));
    } on FileSystemException {
      // Not remembered: calibrate again next time, nothing worse.
    }
  }
}

/// One line of the pattern (mm, y up).
class _Line {
  _Line(this.points, this.closed, this.kind);

  final List<Offset> points;
  final bool closed;
  final String kind;
}

class _Label {
  _Label(this.at, this.text, this.sizeMm);

  final Offset at;
  final String text;
  final double sizeMm;
}

/// The pattern full screen, at 1:1 once calibrated.
class ProjectorPage extends StatefulWidget {
  const ProjectorPage({super.key, required this.drawing, required this.memory});

  /// `Engine.projectorDrawing`: lines, labels and bounds in mm.
  final Map<String, dynamic> drawing;
  final ProjectorMemory memory;

  @override
  State<ProjectorPage> createState() => ProjectorPageState();
}

class ProjectorPageState extends State<ProjectorPage> {
  late ProjectorSettings settings = widget.memory.read();

  /// How far the pattern is moved from the middle of the screen (mm,
  /// screen directions, y up).
  Offset pan = Offset.zero;
  bool calibrating = false;
  final _focus = FocusNode();
  final _width = TextEditingController();
  final _height = TextEditingController();

  late final List<_Line> _lines = [
    for (final l in (widget.drawing['lines'] as List? ?? const []))
      _Line(
        [
          for (var i = 0; i + 1 < (l['points'] as List).length; i += 2)
            Offset(
              ((l['points'] as List)[i] as num).toDouble(),
              ((l['points'] as List)[i + 1] as num).toDouble(),
            ),
        ],
        l['closed'] == true,
        l['kind'] as String? ?? 'cut',
      ),
  ];

  late final List<_Label> _labels = [
    for (final l in (widget.drawing['labels'] as List? ?? const []))
      _Label(
        Offset((l['x'] as num).toDouble(), (l['y'] as num).toDouble()),
        l['text'] as String? ?? '',
        (l['size_mm'] as num?)?.toDouble() ?? 8,
      ),
  ];

  /// The middle of the pieces (mm).
  late final Offset _center = () {
    final b = widget.drawing['bounds'] as List? ?? const [0, 0, 0, 0];
    double at(int i) => (b[i] as num).toDouble();
    return Offset((at(0) + at(2)) / 2, (at(1) + at(3)) / 2);
  }();

  void _set(ProjectorSettings s) {
    setState(() => settings = s);
    widget.memory.write(s);
  }

  void mirror() => _set(settings.copyWith(mirror: !settings.mirror));
  void turn() =>
      _set(settings.copyWith(quarterTurns: settings.quarterTurns + 1));
  void contrast() => _set(settings.copyWith(dark: !settings.dark));

  void startCalibration() => setState(() {
    calibrating = true;
    _width.text = '';
    _height.text = '';
  });

  /// Apply the sides measured on the table (cm).
  void calibrate(double widthCm, double heightCm) {
    if (!(widthCm > 0) || !(heightCm > 0)) return;
    _set(settings.calibrated(widthCm * 10, heightCm * 10));
    setState(() => calibrating = false);
    _focus.requestFocus();
  }

  KeyEventResult _key(FocusNode _, KeyEvent e) {
    if (e is! KeyDownEvent && e is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    if (calibrating) {
      if (e.logicalKey == LogicalKeyboardKey.escape) {
        setState(() => calibrating = false);
        return KeyEventResult.handled;
      }
      return KeyEventResult.ignored;
    }
    final step = HardwareKeyboard.instance.isShiftPressed ? 100.0 : 10.0;
    final k = e.logicalKey;
    void move(Offset by) => setState(() => pan += by);
    if (k == LogicalKeyboardKey.arrowLeft) {
      move(Offset(-step, 0));
    } else if (k == LogicalKeyboardKey.arrowRight) {
      move(Offset(step, 0));
    } else if (k == LogicalKeyboardKey.arrowUp) {
      move(Offset(0, step));
    } else if (k == LogicalKeyboardKey.arrowDown) {
      move(Offset(0, -step));
    } else if (k == LogicalKeyboardKey.home) {
      setState(() => pan = Offset.zero);
    } else if (k == LogicalKeyboardKey.escape) {
      Navigator.of(context).maybePop();
    } else if (k == LogicalKeyboardKey.keyM) {
      mirror();
    } else if (k == LogicalKeyboardKey.keyR) {
      turn();
    } else if (k == LogicalKeyboardKey.keyI) {
      contrast();
    } else if (k == LogicalKeyboardKey.keyC) {
      startCalibration();
    } else {
      return KeyEventResult.ignored;
    }
    return KeyEventResult.handled;
  }

  @override
  void dispose() {
    _focus.dispose();
    _width.dispose();
    _height.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    final ink = settings.dark ? Colors.white : Colors.black;
    final paper = settings.dark ? Colors.black : Colors.white;
    Widget tool(String key, IconData icon, String tip, VoidCallback onTap) =>
        IconButton(
          key: Key(key),
          tooltip: tip,
          onPressed: onTap,
          icon: Icon(icon, color: ink.withValues(alpha: 0.75), size: 20),
        );
    return Focus(
      focusNode: _focus,
      autofocus: true,
      onKeyEvent: _key,
      child: Material(
        color: paper,
        child: Stack(
          children: [
            Positioned.fill(
              child: GestureDetector(
                key: const Key('projector-canvas'),
                onPanUpdate: calibrating
                    ? null
                    : (d) => setState(
                        () => pan += Offset(
                          d.delta.dx / settings.pxPerMmX,
                          -d.delta.dy / settings.pxPerMmY,
                        ),
                      ),
                child: CustomPaint(
                  painter: _ProjectorPainter(
                    lines: calibrating ? const [] : _lines,
                    labels: calibrating ? const [] : _labels,
                    center: _center,
                    pan: pan,
                    settings: settings,
                    ink: ink,
                    paper: paper,
                    square: calibrating,
                    scaleLabel: '${(_scaleBarMm / 10).round()} ${l.cm}',
                  ),
                ),
              ),
            ),
            if (_lines.isEmpty && !calibrating)
              Center(
                child: Text(
                  l.projectorNothing,
                  style: ts(fontSize: 16, color: ink),
                ),
              ),
            Positioned(
              top: 8,
              left: 12,
              right: 12,
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      l.projectorHelp,
                      style: ts(
                        fontSize: 12,
                        color: ink.withValues(alpha: 0.6),
                      ),
                    ),
                  ),
                  tool(
                    'projector-mirror',
                    Icons.flip_rounded,
                    l.projectorMirror,
                    mirror,
                  ),
                  tool(
                    'projector-turn',
                    Icons.rotate_90_degrees_ccw_rounded,
                    l.projectorTurn,
                    turn,
                  ),
                  tool(
                    'projector-contrast',
                    Icons.contrast_rounded,
                    l.projectorContrast,
                    contrast,
                  ),
                  tool(
                    'projector-calibrate',
                    Icons.straighten_rounded,
                    l.projectorCalibrate,
                    startCalibration,
                  ),
                  tool(
                    'projector-close',
                    Icons.close_rounded,
                    l.close,
                    () => Navigator.of(context).maybePop(),
                  ),
                ],
              ),
            ),
            if (calibrating)
              Positioned(
                left: 16,
                bottom: 16,
                child: _CalibrationCard(
                  width: _width,
                  height: _height,
                  onApply: calibrate,
                  onReset: () {
                    _set(
                      settings.copyWith(
                        pxPerMmX: defaultPxPerMm,
                        pxPerMmY: defaultPxPerMm,
                      ),
                    );
                    setState(() => calibrating = false);
                    _focus.requestFocus();
                  },
                  onCancel: () {
                    setState(() => calibrating = false);
                    _focus.requestFocus();
                  },
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// Type the sides of the projected square, measured on the table.
class _CalibrationCard extends StatelessWidget {
  const _CalibrationCard({
    required this.width,
    required this.height,
    required this.onApply,
    required this.onReset,
    required this.onCancel,
  });

  final TextEditingController width;
  final TextEditingController height;
  final void Function(double widthCm, double heightCm) onApply;
  final VoidCallback onReset;
  final VoidCallback onCancel;

  double? _parse(String s) => double.tryParse(s.trim().replaceAll(',', '.'));

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    void apply() {
      final w = _parse(width.text);
      final h = _parse(height.text);
      if (w != null && h != null) onApply(w, h);
    }

    Widget field(String key, String label, TextEditingController c) => SizedBox(
      width: 150,
      child: TextField(
        key: Key(key),
        controller: c,
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        decoration: InputDecoration(
          labelText: label,
          suffixText: l.cm,
          isDense: true,
        ),
        onSubmitted: (_) => apply(),
      ),
    );
    return Container(
      width: 360,
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        color: p.surface,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        border: Border.all(color: p.border),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l.projectorCalibrate,
            style: ts(fontSize: 15, fontWeight: FontWeight.w600, color: p.text),
          ),
          const SizedBox(height: Space.xs),
          Text(
            l.projectorCalibrateHint,
            style: ts(fontSize: 12.5, height: 1.4, color: p.textMuted),
          ),
          const SizedBox(height: Space.s),
          Row(
            children: [
              field(
                'projector-measured-width',
                l.projectorMeasuredWidth,
                width,
              ),
              const SizedBox(width: Space.s),
              field(
                'projector-measured-height',
                l.projectorMeasuredHeight,
                height,
              ),
            ],
          ),
          const SizedBox(height: Space.s),
          Row(
            children: [
              TextButton(onPressed: onReset, child: Text(l.projectorReset)),
              const Spacer(),
              TextButton(onPressed: onCancel, child: Text(l.cancel)),
              const SizedBox(width: Space.xs),
              FilledButton(
                key: const Key('projector-apply'),
                onPressed: apply,
                child: Text(l.projectorApply),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _ProjectorPainter extends CustomPainter {
  _ProjectorPainter({
    required this.lines,
    required this.labels,
    required this.center,
    required this.pan,
    required this.settings,
    required this.ink,
    required this.paper,
    required this.square,
    required this.scaleLabel,
  });

  final List<_Line> lines;
  final List<_Label> labels;
  final Offset center;
  final Offset pan;
  final ProjectorSettings settings;
  final Color ink;
  final Color paper;

  /// Draw the calibration square instead of the pattern.
  final bool square;
  final String scaleLabel;

  /// A pattern point (mm, y up) on the screen.
  Offset _place(Offset p, Size size) {
    var v = p - center;
    for (var i = 0; i < settings.quarterTurns; i++) {
      v = Offset(-v.dy, v.dx);
    }
    if (settings.mirror) v = Offset(-v.dx, v.dy);
    v += pan;
    return Offset(
      size.width / 2 + v.dx * settings.pxPerMmX,
      size.height / 2 - v.dy * settings.pxPerMmY,
    );
  }

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = paper);
    final stroke = Paint()
      ..color = ink
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    if (square) {
      final w = calibrationMm * settings.pxPerMmX;
      final h = calibrationMm * settings.pxPerMmY;
      canvas.drawRect(
        Rect.fromCenter(center: size.center(Offset.zero), width: w, height: h),
        stroke..strokeWidth = 2,
      );
    }
    for (final line in lines) {
      if (line.points.length < 2) continue;
      final pts = [for (final p in line.points) _place(p, size)];
      if (line.closed) pts.add(pts.first);
      final (width, dash) = switch (line.kind) {
        'cut' || 'notch' => (2.5, const <double>[]),
        'sew' => (1.5, const [12.0, 6.0]),
        'internal' => (1.5, const [16.0, 5.0, 3.0, 5.0]),
        _ => (1.2, const <double>[]),
      };
      stroke.strokeWidth = width;
      if (dash.isEmpty) {
        canvas.drawPath(Path()..addPolygon(pts, false), stroke);
      } else {
        _dashed(canvas, pts, dash, stroke);
      }
    }
    for (final label in labels) {
      final painter = TextPainter(
        text: TextSpan(
          text: label.text,
          style: TextStyle(
            color: ink,
            fontSize: math.max(10, label.sizeMm * settings.pxPerMmY),
          ),
        ),
        textDirection: TextDirection.ltr,
      )..layout();
      final at = _place(label.at, size);
      painter.paint(canvas, at - Offset(painter.width / 2, painter.height / 2));
    }
    // The scale bar, bottom right: 10 cm across and down.
    final bar = _scaleBarMm;
    final corner = Offset(size.width - 24, size.height - 24);
    stroke.strokeWidth = 2;
    canvas.drawPath(
      Path()
        ..moveTo(corner.dx - bar * settings.pxPerMmX, corner.dy)
        ..lineTo(corner.dx, corner.dy)
        ..lineTo(corner.dx, corner.dy - bar * settings.pxPerMmY),
      stroke,
    );
    final text = TextPainter(
      text: TextSpan(
        text: scaleLabel,
        style: TextStyle(color: ink, fontSize: 12),
      ),
      textDirection: TextDirection.ltr,
    )..layout();
    text.paint(
      canvas,
      Offset(
        corner.dx - bar * settings.pxPerMmX / 2 - text.width / 2,
        corner.dy - text.height - 4,
      ),
    );
  }

  /// A polyline drawn with a dash pattern (on, off, … in pixels).
  void _dashed(Canvas canvas, List<Offset> pts, List<double> dash, Paint p) {
    var k = 0;
    var left = dash[0];
    var on = true;
    final path = Path();
    for (var i = 0; i + 1 < pts.length; i++) {
      var a = pts[i];
      final b = pts[i + 1];
      var seg = (b - a).distance;
      if (seg == 0) continue;
      final dir = (b - a) / seg;
      while (seg > 0) {
        final run = math.min(left, seg);
        final next = a + dir * run;
        if (on) {
          path
            ..moveTo(a.dx, a.dy)
            ..lineTo(next.dx, next.dy);
        }
        a = next;
        seg -= run;
        left -= run;
        if (left <= 1e-9) {
          k = (k + 1) % dash.length;
          left = dash[k];
          on = !on;
        }
      }
    }
    canvas.drawPath(path, p);
  }

  @override
  bool shouldRepaint(_ProjectorPainter old) =>
      old.pan != pan ||
      old.settings != settings ||
      old.square != square ||
      old.lines != lines ||
      old.ink != ink;
}
