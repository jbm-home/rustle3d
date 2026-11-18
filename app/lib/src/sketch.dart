// Sketching on the board (spec 19): constraints and dimensions in the
// inspector and over the pattern, the snap hint, the round shapes.

part of 'shell.dart';

/// A constraint's kind, named for the user (a distance along an axis is a
/// width or a height).
String _kindName(L l, Map<String, dynamic> c) {
  final kind = c['kind'] as String;
  return switch ((kind, c['along'])) {
    ('distance', 'horizontal') => l.constraintKind('width'),
    ('distance', 'vertical') => l.constraintKind('height'),
    _ => l.constraintKind(kind),
  };
}

/// What can be constrained in what is picked on the board: one or two
/// corners, one or two edges, or a corner and an edge.
class _ConstrainCard extends StatelessWidget {
  const _ConstrainCard({
    required this.points,
    required this.edges,
    required this.constraints,
    required this.run,
  });

  final List<PickedPoint> points;
  final List<String> edges;

  /// The piece's constraints (`constraint.list`).
  final List<dynamic> constraints;
  final ValueChanged<Map<String, Object?>> run;

  void _add(Map<String, Object?> c, {bool measure = false}) => run({
    'type': 'constraint.add',
    'constraint': c,
    if (measure) 'measure': true,
  });

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final pts = [for (final q in points) q.id];
    Widget chip(String label, VoidCallback onTap, {IconData? icon}) =>
        ActionChip(
          avatar: icon == null ? null : Icon(icon, size: 16),
          label: Text(label, style: ts(fontSize: 12.5)),
          onPressed: onTap,
        );
    final smooth =
        pts.length == 1 &&
        constraints.any(
          (c) => c['kind'] == 'tangent' && c['point'] == pts.first,
        );
    final List<Widget> chips = switch ((pts.length, edges.length)) {
      (0, 1) => [
        chip(
          l.constraintKind('horizontal'),
          () => _add({'kind': 'horizontal', 'edge': edges[0]}),
        ),
        chip(
          l.constraintKind('vertical'),
          () => _add({'kind': 'vertical', 'edge': edges[0]}),
        ),
        chip(
          l.constraintKind('length'),
          () => _add({
            'kind': 'length',
            'edge': edges[0],
            'mm': 1,
          }, measure: true),
          icon: Icons.straighten_rounded,
        ),
      ],
      (0, 2) => [
        chip(
          l.constraintKind('parallel'),
          () => _add({'kind': 'parallel', 'a': edges[0], 'b': edges[1]}),
        ),
        chip(
          l.constraintKind('perpendicular'),
          () => _add({'kind': 'perpendicular', 'a': edges[0], 'b': edges[1]}),
        ),
        chip(
          l.constraintKind('equal_length'),
          () => _add({'kind': 'equal_length', 'a': edges[0], 'b': edges[1]}),
        ),
        chip(
          l.constraintKind('angle'),
          () => _add({
            'kind': 'angle',
            'a': edges[0],
            'b': edges[1],
            'deg': 90,
          }, measure: true),
          icon: Icons.architecture_rounded,
        ),
      ],
      (1, 0) => [
        chip(
          l.constraintKind('fixed'),
          () => _add({'kind': 'fixed', 'point': pts[0]}),
          icon: Icons.lock_outline_rounded,
        ),
        chip(
          smooth ? l.pointCorner : l.pointSmooth,
          () => run({
            'type': 'point.set_smooth',
            'point': pts[0],
            'smooth': !smooth,
          }),
        ),
        chip(
          l.pointRemove,
          () => run({'type': 'point.remove', 'point': pts[0]}),
          icon: Icons.remove_circle_outline_rounded,
        ),
      ],
      (2, 0) => [
        for (final along in ['direct', 'horizontal', 'vertical'])
          chip(
            l.constraintKind(switch (along) {
              'horizontal' => 'width',
              'vertical' => 'height',
              _ => 'distance',
            }),
            () => _add({
              'kind': 'distance',
              'a': pts[0],
              'b': pts[1],
              'mm': 1,
              'along': along,
            }, measure: true),
            icon: Icons.straighten_rounded,
          ),
        chip(
          l.constraintKind('coincident'),
          () => _add({'kind': 'coincident', 'a': pts[0], 'b': pts[1]}),
        ),
      ],
      (1, 1) => [
        chip(
          l.constraintKind('on_edge'),
          () => _add({'kind': 'on_edge', 'point': pts[0], 'edge': edges[0]}),
        ),
      ],
      _ => const [],
    };
    if (chips.isEmpty) return const SizedBox.shrink();
    return Container(
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        color: p.surfaceMuted,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l.constrain,
            style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
          ),
          const SizedBox(height: Space.s),
          // Keys stay with the board (Delete, Enter, the tools).
          ExcludeFocus(
            child: Wrap(
              spacing: Space.xs,
              runSpacing: Space.xs,
              children: chips,
            ),
          ),
        ],
      ),
    );
  }
}

/// The piece's constraints and dimensions: each removable, a dimension's
/// value typed in; and how constrained the piece is.
class _ConstraintsCard extends StatelessWidget {
  const _ConstraintsCard({
    required this.constraints,
    required this.groups,
    required this.run,
  });

  final List<dynamic> constraints;
  final List<dynamic> groups;
  final ValueChanged<Map<String, Object?>> run;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final freedom = groups.fold<int>(
      0,
      (n, g) => n + ((g['freedom'] as num?)?.toInt() ?? 0),
    );
    final redundant = groups.any(
      (g) => ((g['redundant'] as List?) ?? const []).isNotEmpty,
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l.constraints,
          style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Text(
          constraints.isEmpty
              ? context.keys(l.constraintsEmpty)
              : l.constraintsFreedom(freedom),
          style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
        ),
        if (redundant)
          Padding(
            padding: const EdgeInsets.only(top: Space.xs),
            child: Text(
              l.constraintsRedundant,
              style: ts(fontSize: 12.5, color: p.danger),
            ),
          ),
        for (final c in constraints.cast<Map<String, dynamic>>())
          Padding(
            padding: const EdgeInsets.only(top: Space.s),
            child: Row(
              children: [
                SizedBox(
                  width: 22,
                  height: 22,
                  child: CustomPaint(
                    painter: _GlyphPainter(
                      c['kind'] as String,
                      c['holds'] == true ? p.accent : p.danger,
                    ),
                  ),
                ),
                const SizedBox(width: Space.s),
                Expanded(
                  child: Tooltip(
                    message: c['holds'] == true ? '' : l.constraintBroken,
                    child: Text(
                      _kindName(l, c),
                      style: ts(
                        fontSize: 13,
                        color: c['holds'] == true ? p.text : p.danger,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ),
                if (c['kind'] case 'distance' || 'length' || 'angle')
                  SizedBox(
                    width: 84,
                    child: _DimensionField(
                      key: ValueKey('${c['id']}-${c['mm'] ?? c['deg']}'),
                      constraint: c,
                      run: run,
                    ),
                  ),
                _IconAction(
                  icon: Icons.close_rounded,
                  tip: l.removeConstraint,
                  onTap: () =>
                      run({'type': 'constraint.remove', 'constraint': c['id']}),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

/// A dimension's value, typed: cm for lengths, degrees for angles.
class _DimensionField extends StatefulWidget {
  const _DimensionField({
    super.key,
    required this.constraint,
    required this.run,
  });

  final Map<String, dynamic> constraint;
  final ValueChanged<Map<String, Object?>> run;

  @override
  State<_DimensionField> createState() => _DimensionFieldState();
}

class _DimensionFieldState extends State<_DimensionField> {
  late final _text = TextEditingController();
  final _focus = FocusNode();

  bool get _angle => widget.constraint['kind'] == 'angle';

  /// The value as shown: cm, or degrees.
  double get _value => _angle
      ? (widget.constraint['deg'] as num).toDouble()
      : (widget.constraint['mm'] as num).toDouble() / 10;

  @override
  void initState() {
    super.initState();
    _focus.addListener(() {
      if (!_focus.hasFocus) _submit();
    });
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!_focus.hasFocus) _text.text = context.decimal(_value, _angle ? 1 : 1);
  }

  @override
  void dispose() {
    _text.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _submit() {
    final typed = _parseNumber(_text.text);
    if (typed == null || typed <= 0 || typed == _value) {
      _text.text = context.decimal(_value, 1);
      return;
    }
    widget.run({
      'type': 'constraint.set_value',
      'constraint': widget.constraint['id'],
      'value': _angle ? typed : typed * 10,
    });
  }

  @override
  Widget build(BuildContext context) => _NumberField(
    controller: _text,
    focus: _focus,
    unit: _angle ? '°' : context.l.cm,
    // Leaving the field applies the value, and gives the keys back to
    // the board's tools.
    onSubmitted: () =>
        _focus.unfocus(disposition: UnfocusDisposition.previouslyFocusedChild),
  );
}

/// A constraint's small sign: drawn, so it reads the same in every font.
class _GlyphPainter extends CustomPainter {
  const _GlyphPainter(this.kind, this.color);

  final String kind;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) =>
      _glyph(canvas, size.center(Offset.zero), kind, color, size.width / 2);

  @override
  bool shouldRepaint(_GlyphPainter old) =>
      old.kind != kind || old.color != color;
}

/// Draw a constraint's sign centred on `at`, `r` its radius.
void _glyph(Canvas canvas, Offset at, String kind, Color color, double r) {
  final pen = Paint()
    ..color = color
    ..strokeWidth = 1.6
    ..strokeCap = StrokeCap.round
    ..style = PaintingStyle.stroke;
  final s = r * 0.5;
  void line(double x0, double y0, double x1, double y1) => canvas.drawLine(
    at + Offset(x0 * s, y0 * s),
    at + Offset(x1 * s, y1 * s),
    pen,
  );
  switch (kind) {
    case 'horizontal':
      line(-1, 0, 1, 0);
    case 'vertical':
      line(0, -1, 0, 1);
    case 'parallel':
      line(-1, 0.8, -0.2, -0.8);
      line(0.2, 0.8, 1, -0.8);
    case 'perpendicular':
      line(-1, 0.8, 1, 0.8);
      line(0, 0.8, 0, -1);
    case 'equal_length':
      line(-1, -0.4, 1, -0.4);
      line(-1, 0.4, 1, 0.4);
    case 'fixed':
      canvas.drawRect(
        Rect.fromCenter(center: at, width: s * 1.6, height: s * 1.6),
        Paint()..color = color,
      );
    case 'tangent':
      canvas.drawCircle(at + Offset(0, -s * 0.4), s * 0.6, pen);
      line(-1, 0.2, 1, 0.2);
    case 'coincident':
      canvas.drawCircle(at, s * 0.9, pen);
      canvas.drawCircle(at, s * 0.3, Paint()..color = color);
    case 'on_edge':
      line(-1, 0.6, 1, 0.6);
      canvas.drawCircle(
        at + Offset(0, s * 0.1),
        s * 0.4,
        Paint()..color = color,
      );
    case 'angle':
      line(-0.9, 0.8, 1, 0.8);
      line(-0.9, 0.8, 0.5, -0.9);
    default:
      // Distances and lengths.
      line(-1, 0, 1, 0);
      line(-1, -0.6, -1, 0.6);
      line(1, -0.6, 1, 0.6);
  }
}

/// Text in a pill over the board, centred on `at`.
void _pill(Canvas canvas, Offset at, String text, Color fg, Color bg) {
  final painter = TextPainter(
    text: TextSpan(
      text: text,
      style: ts(fontSize: 11.5, fontWeight: FontWeight.w600, color: fg),
    ),
    textDirection: TextDirection.ltr,
  )..layout();
  final box = Rect.fromCenter(
    center: at,
    width: painter.width + 12,
    height: painter.height + 6,
  );
  canvas.drawRRect(
    RRect.fromRectAndRadius(box, const Radius.circular(999)),
    Paint()..color = bg,
  );
  painter.paint(canvas, box.topLeft + const Offset(6, 3));
  painter.dispose();
}

/// The constraints of the selected piece over the board: a sign at each
/// edge or point they name, dimensions with their value.
void _paintConstraints(
  BuildContext context,
  Canvas canvas,
  Offset Function(Offset mm) toScreen,
  List<dynamic> constraints,
  Palette p,
) {
  Offset at(Object? xy) => toScreen(_ShellState._at(xy));
  // Several signs on one spot sit side by side.
  final taken = <Offset, int>{};
  Offset slot(Offset where) {
    final key = Offset(
      (where.dx / 4).roundToDouble(),
      (where.dy / 4).roundToDouble(),
    );
    final n = taken[key] ?? 0;
    taken[key] = n + 1;
    return where + Offset(14.0 + 22 * n, -14);
  }

  for (final c in constraints.cast<Map<String, dynamic>>()) {
    final kind = c['kind'] as String;
    final color = c['holds'] == true ? p.accent : p.danger;
    final marks = [for (final m in (c['marks'] as List? ?? const [])) at(m)];
    if (marks.isEmpty) continue;
    if (kind == 'distance' && marks.length == 2) {
      final dash = Paint()
        ..color = color.withValues(alpha: 0.7)
        ..strokeWidth = 1;
      final (a, b) = switch (c['along']) {
        'horizontal' => (marks[0], Offset(marks[1].dx, marks[0].dy)),
        'vertical' => (marks[0], Offset(marks[0].dx, marks[1].dy)),
        _ => (marks[0], marks[1]),
      };
      final n = ((b - a).distance / 6).floor();
      for (var i = 0; i < n; i += 2) {
        canvas.drawLine(
          Offset.lerp(a, b, i / n)!,
          Offset.lerp(a, b, (i + 1) / n)!,
          dash,
        );
      }
      _pill(
        canvas,
        Offset.lerp(a, b, 0.5)!,
        '${context.number((c['mm'] as num) / 10, 1)} ${context.l.cm}',
        p.surface,
        color,
      );
      continue;
    }
    if (kind == 'length' || kind == 'angle') {
      final where = kind == 'angle' && marks.length == 2
          ? Offset.lerp(marks[0], marks[1], 0.5)!
          : marks.first;
      final text = kind == 'angle'
          ? '${context.number(c['deg'] as num, 1)}°'
          : '${context.number((c['mm'] as num) / 10, 1)} ${context.l.cm}';
      _pill(canvas, slot(where), text, p.surface, color);
      continue;
    }
    for (final m in marks) {
      final o = slot(m);
      canvas.drawCircle(o, 9, Paint()..color = p.surface);
      canvas.drawCircle(
        o,
        9,
        Paint()
          ..color = color
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1,
      );
      _glyph(canvas, o, kind, color, 8);
    }
  }
}

/// Where the pointer snaps: a ring on the spot, what it snapped to, and a
/// dashed line to the point it is level with.
void _paintSnap(
  BuildContext context,
  Canvas canvas,
  Offset Function(Offset mm) toScreen,
  Snap s,
  Palette p,
) {
  if (s.kind == 'free') return;
  final at = toScreen(Offset(s.xMm, s.yMm));
  final pen = Paint()
    ..color = p.accent
    ..strokeWidth = 1.5
    ..style = PaintingStyle.stroke;
  if (s.from case final f? when f.length == 2) {
    final from = toScreen(Offset(f[0], f[1]));
    final n = ((at - from).distance / 6).floor();
    for (var i = 0; i < n; i += 2) {
      canvas.drawLine(
        Offset.lerp(from, at, i / n)!,
        Offset.lerp(from, at, (i + 1) / n)!,
        pen..strokeWidth = 1,
      );
    }
  }
  canvas.drawCircle(at, 7, pen..strokeWidth = 1.5);
  _pill(
    canvas,
    at + const Offset(0, 22),
    context.l.snapKind(s.kind),
    p.surface,
    p.accent,
  );
}

/// Ask for numbers (and maybe one choice among a few): a small dialog with
/// a field per number. Returns them, or null when cancelled.
Future<(List<double>, int)?> _askNumbers(
  BuildContext context, {
  required String title,
  required List<(String, String, double)> fields,
  List<String> choices = const [],
  String? hint,
}) {
  final controllers = [
    for (final (_, _, v) in fields)
      TextEditingController(text: context.decimal(v, 1)),
  ];
  var choice = 0;
  return showDialog<(List<double>, int)>(
    context: context,
    builder: (context) => StatefulBuilder(
      builder: (context, setState) {
        final l = context.l;
        void done() {
          final values = [for (final c in controllers) _parseNumber(c.text)];
          if (values.any((v) => v == null)) return;
          Navigator.pop(context, ([for (final v in values) v!], choice));
        }

        return AlertDialog(
          title: Text(title),
          content: SizedBox(
            width: 320,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (choices.isNotEmpty) ...[
                  SegmentedButton<int>(
                    segments: [
                      for (final (i, c) in choices.indexed)
                        ButtonSegment(value: i, label: Text(c)),
                    ],
                    selected: {choice},
                    onSelectionChanged: (s) => setState(() => choice = s.first),
                  ),
                  const SizedBox(height: Space.m),
                ],
                for (final ((label, unit, _), c) in fields.indexed.map(
                  (e) => (e.$2, controllers[e.$1]),
                ))
                  Padding(
                    padding: const EdgeInsets.only(bottom: Space.s),
                    child: Row(
                      children: [
                        Expanded(child: Text(label)),
                        SizedBox(
                          width: 110,
                          child: _NumberField(
                            controller: c,
                            unit: unit,
                            signed: true,
                            onSubmitted: done,
                          ),
                        ),
                      ],
                    ),
                  ),
                if (hint != null)
                  Text(
                    hint,
                    style: ts(
                      fontSize: 12.5,
                      height: 1.4,
                      color: Tokens.of(context).textMuted,
                    ),
                  ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: Text(l.cancel),
            ),
            FilledButton(onPressed: done, child: Text(l.create)),
          ],
        );
      },
    ),
  );
}
