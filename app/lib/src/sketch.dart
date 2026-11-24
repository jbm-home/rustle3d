// Sketching on the board (spec 19): constraints and dimensions in the
// inspector and over the pattern, the snap hint, the round shapes.

part of 'shell.dart';

/// Whether a constraint (as `constraint.list` gives it) is a dimension.
bool _isDimension(Map<String, dynamic> c) =>
    c.containsKey('mm') || c.containsKey('deg');

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

//// The piece's constraints and dimensions: each removable, a dimension's
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
        for (final c in constraints.cast<Map<String, dynamic>>()) ...[
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
                    message: c['holds'] != true
                        ? l.constraintBroken
                        : _isDimension(c)
                        ? l.dimensionRename
                        : '',
                    // A dimension's name (or kind): click to name it.
                    child: InkWell(
                      key: ValueKey('name-${c['id']}'),
                      onTap: !_isDimension(c)
                          ? null
                          : () async {
                              final n = await _askName(
                                context,
                                c['name'] as String? ?? '',
                              );
                              if (n == null) return;
                              run({
                                'type': 'constraint.rename',
                                'constraint': c['id'],
                                if (n.isNotEmpty) 'name': n,
                              });
                            },
                      child: Text(
                        (c['name'] as String?) ?? _kindName(l, c),
                        style: ts(
                          fontSize: 13,
                          color: c['holds'] == true ? p.text : p.danger,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ),
                ),
                if (_isDimension(c)) ...[
                  SizedBox(
                    width: 84,
                    child: _DimensionField(
                      key: ValueKey('${c['id']}-${c['mm'] ?? c['deg']}'),
                      constraint: c,
                      run: run,
                    ),
                  ),
                  _IconAction(
                    key: ValueKey('formula-${c['id']}'),
                    icon: Icons.functions_rounded,
                    tip: l.formulaTip,
                    onTap: () async {
                      final f = await _askFormula(
                        context,
                        c['formula'] as String? ?? '',
                      );
                      if (f == null) return;
                      run({
                        'type': 'constraint.set_formula',
                        'constraint': c['id'],
                        if (f.isNotEmpty) 'formula': f,
                      });
                    },
                  ),
                ],
                _IconAction(
                  icon: Icons.close_rounded,
                  tip: l.removeConstraint,
                  onTap: () =>
                      run({'type': 'constraint.remove', 'constraint': c['id']}),
                ),
              ],
            ),
          ),
          if (c['formula'] case final String f)
            Padding(
              padding: const EdgeInsets.only(left: 30),
              child: Text(
                '= $f',
                style: ts(fontSize: 12, color: p.textMuted),
                overflow: TextOverflow.ellipsis,
              ),
            ),
        ],
      ],
    );
  }
}

/// A dimension's value, typed: cm for lengths, degrees for angles, or a
/// formula (`waist / 4 + 1 cm`).
class _DimensionField extends StatefulWidget {
  const _DimensionField({
    super.key,
    required this.constraint,
    required this.run,
    this.autofocus = false,
    this.onDone,
  });

  final Map<String, dynamic> constraint;
  final ValueChanged<Map<String, Object?>> run;
  final bool autofocus;

  /// Called once the field is left (the value applied, or Esc).
  final VoidCallback? onDone;

  @override
  State<_DimensionField> createState() => _DimensionFieldState();
}

class _DimensionFieldState extends State<_DimensionField> {
  late final _text = TextEditingController();
  final _focus = FocusNode();

  /// Esc leaves the value as it was.
  bool _cancelled = false;

  bool get _angle => widget.constraint['kind'] == 'angle';

  /// The value as shown: cm, or degrees.
  double get _value => _angle
      ? (widget.constraint['deg'] as num).toDouble()
      : (widget.constraint['mm'] as num).toDouble() / 10;

  @override
  void initState() {
    super.initState();
    _focus.addListener(() {
      if (_focus.hasFocus) return;
      if (!_cancelled) _submit();
      _cancelled = false;
      widget.onDone?.call();
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
    final text = _text.text.trim();
    final typed = _parseNumber(text);
    if (typed == null && text.isNotEmpty) {
      // Not a number: a formula.
      widget.run({
        'type': 'constraint.set_formula',
        'constraint': widget.constraint['id'],
        'formula': text,
      });
      return;
    }
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

  void _leave() =>
      _focus.unfocus(disposition: UnfocusDisposition.previouslyFocusedChild);

  @override
  Widget build(BuildContext context) => CallbackShortcuts(
    bindings: {
      const SingleActivator(LogicalKeyboardKey.escape): () {
        _cancelled = true;
        _leave();
      },
    },
    child: _NumberField(
      controller: _text,
      focus: _focus,
      formula: true,
      autofocus: widget.autofocus,
      unit: _angle ? '°' : context.l.cm,
      // Leaving the field applies the value, and gives the keys back to
      // the board's tools.
      onSubmitted: _leave,
    ),
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

//// Text in a pill over the board, centred on `at`. Returns where it is.
Rect _pill(Canvas canvas, Offset at, String text, Color fg, Color bg) {
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
  return box;
}

/// The kinds drawn as a dimension line between two marks.
const _lineDimensions = {'distance', 'notch_distance', 'guide_distance'};

/// The two ends of a dimension line (mm): a distance's points (across or
/// up and down, the second level with the first), a notch's edge end and
/// the notch, a corner and its foot on a line.
(Offset, Offset) _dimensionLine(Map<String, dynamic> c, List<Offset> marks) {
  final (a, b) = (marks[0], marks[1]);
  return switch (c['along']) {
    'horizontal' => (a, Offset(b.dx, a.dy)),
    'vertical' => (a, Offset(a.dx, b.dy)),
    _ => (a, b),
  };
}

/// Where a dimension's label is drawn by default (mm), from its marks:
/// the middle of its line, of its edge, between its two edges.
Offset? _labelAnchor(Map<String, dynamic> c, List<Offset> marks) {
  if (marks.isEmpty) return null;
  final kind = c['kind'] as String;
  if (_lineDimensions.contains(kind) && marks.length == 2) {
    final (a, b) = _dimensionLine(c, marks);
    return Offset.lerp(a, b, 0.5);
  }
  if (kind == 'angle' && marks.length == 2) {
    return Offset.lerp(marks[0], marks[1], 0.5);
  }
  return marks.first;
}

/// A dimension's label: its name, if any, and its value.
String _labelText(BuildContext context, Map<String, dynamic> c) {
  final value = c['kind'] == 'angle'
      ? '${context.number(c['deg'] as num, 1)}°'
      : '${context.number((c['mm'] as num) / 10, 1)} ${context.l.cm}';
  return switch (c['name']) {
    final String name => '$name  $value',
    _ => value,
  };
}

/// Every piece's constraints over the board (the selected pieces' in
/// full, the others fainter): a sign at each edge or point they name,
/// dimensions with their value where their label is placed. Returns each
/// dimension's label and where it was drawn (board pixels).
List<(Map<String, dynamic>, Rect)> _paintConstraints(
  BuildContext context,
  Canvas canvas,
  Offset Function(Offset mm) toScreen,
  List<dynamic> constraints,
  List<String> selection,
  Palette p,
) {
  final labels = <(Map<String, dynamic>, Rect)>[];
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
    final faint = !selection.contains(c['panel']);
    final color = (c['holds'] == true ? p.accent : p.danger).withValues(
      alpha: faint ? 0.5 : 1,
    );
    final marks = [
      for (final m in (c['marks'] as List? ?? const [])) _ShellState._at(m),
    ];
    if (marks.isEmpty) continue;
    if (!_isDimension(c)) {
      for (final m in marks) {
        final o = slot(toScreen(m));
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
      continue;
    }
    final anchor = _labelAnchor(c, marks)!;
    final offset = c['label'] == null ? null : _ShellState._at(c['label']);
    final thin = Paint()
      ..color = color.withValues(alpha: 0.7)
      ..strokeWidth = 1;
    void dashed(Offset a, Offset b) {
      final n = ((b - a).distance / 6).floor();
      for (var i = 0; i < n; i += 2) {
        canvas.drawLine(
          Offset.lerp(a, b, i / n)!,
          Offset.lerp(a, b, (i + 1) / n)!,
          thin,
        );
      }
    }

    Offset where;
    if (_lineDimensions.contains(kind) && marks.length == 2) {
      // The line moves out with its label, across itself.
      final (a, b) = _dimensionLine(c, marks);
      var shift = Offset.zero;
      if (offset != null && (b - a).distance > 1e-9) {
        final u = (b - a) / (b - a).distance;
        final n = Offset(-u.dy, u.dx);
        shift = n * (offset.dx * n.dx + offset.dy * n.dy);
      }
      if (shift != Offset.zero) {
        canvas.drawLine(toScreen(marks[0]), toScreen(a + shift), thin);
        canvas.drawLine(toScreen(marks[1]), toScreen(b + shift), thin);
      } else if (kind == 'distance' && c['along'] != 'direct') {
        canvas.drawLine(toScreen(marks[1]), toScreen(b), thin);
      }
      dashed(toScreen(a + shift), toScreen(b + shift));
      where = toScreen(anchor + (offset ?? Offset.zero));
    } else if (offset != null) {
      where = toScreen(anchor + offset);
      dashed(toScreen(anchor), where);
    } else {
      where = slot(toScreen(anchor));
    }
    labels.add((
      c,
      _pill(canvas, where, _labelText(context, c), p.surface, color),
    ));
  }
  return labels;
}

// Where the pointer snaps: a ring on the spot, what it snapped to, and a
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

/// Ask for a dimension's formula (empty: no formula). Returns it, or null
/// when cancelled.
/// Ask for a dimension's name ('' for none; null when cancelled).
Future<String?> _askName(BuildContext context, String current) {
  final text = TextEditingController(text: current);
  return showDialog<String>(
    context: context,
    builder: (context) {
      final l = context.l;
      final p = Tokens.of(context);
      return AlertDialog(
        title: Text(l.dimensionNameTitle),
        content: SizedBox(
          width: 360,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              TextField(
                key: const Key('dimension_name'),
                controller: text,
                autofocus: true,
                decoration: const InputDecoration(
                  hintText: 'armhole',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                onSubmitted: (v) => Navigator.pop(context, v.trim()),
              ),
              const SizedBox(height: Space.s),
              Text(
                l.dimensionNameHelp,
                style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, text.text.trim()),
            child: Text(l.apply),
          ),
        ],
      );
    },
  );
}

Future<String?> _askFormula(BuildContext context, String current) {
  final text = TextEditingController(text: current);
  return showDialog<String>(
    context: context,
    builder: (context) {
      final l = context.l;
      final p = Tokens.of(context);
      return AlertDialog(
        title: Text(l.formulaTitle),
        content: SizedBox(
          width: 360,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              TextField(
                key: const Key('formula'),
                controller: text,
                autofocus: true,
                decoration: const InputDecoration(
                  hintText: 'waist / 4 + 10 mm',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                onSubmitted: (v) => Navigator.pop(context, v.trim()),
              ),
              const SizedBox(height: Space.s),
              Text(
                l.formulaHelp,
                style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, text.text.trim()),
            child: Text(l.apply),
          ),
        ],
      );
    },
  );
}

/// The project's named variables for formulas: each with its formula and
/// value, editable.
class _VariablesCard extends StatelessWidget {
  const _VariablesCard({required this.query, required this.run});

  final dynamic Function(Map<String, Object?>) query;
  final ValueChanged<Map<String, Object?>> run;

  /// Ask for a variable's name and formula.
  Future<(String, String)?> _ask(
    BuildContext context, {
    String name = '',
    String formula = '',
  }) {
    final n = TextEditingController(text: name);
    final f = TextEditingController(text: formula);
    return showDialog<(String, String)>(
      context: context,
      builder: (context) {
        final l = context.l;
        void done() => Navigator.pop(context, (n.text.trim(), f.text.trim()));
        return AlertDialog(
          title: Text(l.variable),
          content: SizedBox(
            width: 340,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  key: const Key('variable-name'),
                  controller: n,
                  autofocus: name.isEmpty,
                  decoration: InputDecoration(
                    labelText: l.variableName,
                    hintText: 'ease',
                    isDense: true,
                  ),
                ),
                const SizedBox(height: Space.s),
                TextField(
                  key: const Key('variable-formula'),
                  controller: f,
                  autofocus: name.isNotEmpty,
                  decoration: InputDecoration(
                    labelText: l.formulaTitle,
                    hintText: '2 cm',
                    isDense: true,
                  ),
                  onSubmitted: (_) => done(),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: Text(l.cancel),
            ),
            FilledButton(onPressed: done, child: Text(l.apply)),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final list = query({'type': 'variable.list'}) as Map<String, dynamic>?;
    final variables = (list?['variables'] as List?) ?? const [];
    Future<void> edit({String name = '', String formula = ''}) async {
      final got = await _ask(context, name: name, formula: formula);
      if (got == null || got.$1.isEmpty || got.$2.isEmpty) return;
      run({'type': 'variable.set', 'name': got.$1, 'formula': got.$2});
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                l.variables,
                style: ts(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                  color: p.text,
                ),
              ),
            ),
            _IconAction(
              key: const Key('add-variable'),
              icon: Icons.add_rounded,
              tip: l.variableAdd,
              onTap: edit,
            ),
          ],
        ),
        const SizedBox(height: Space.xs),
        Text(
          l.variablesHint,
          style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
        ),
        for (final v in variables.cast<Map<String, dynamic>>())
          Padding(
            padding: const EdgeInsets.only(top: Space.s),
            child: Row(
              children: [
                Expanded(
                  child: InkWell(
                    onTap: () => edit(
                      name: v['name'] as String,
                      formula: v['formula'] as String,
                    ),
                    child: Text(
                      '${v['name']} = ${v['formula']}',
                      style: ts(fontSize: 13, color: p.text),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ),
                if (v['mm'] case final num mm)
                  Text(
                    '${context.number(mm / 10, 1)} ${l.cm}',
                    style: ts(
                      fontSize: 12.5,
                      color: p.textMuted,
                      tabular: true,
                    ),
                  )
                else if (v['number'] case final num n)
                  Text(
                    context.number(n, 2),
                    style: ts(
                      fontSize: 12.5,
                      color: p.textMuted,
                      tabular: true,
                    ),
                  ),
                _IconAction(
                  icon: Icons.close_rounded,
                  tip: l.removeConstraint,
                  onTap: () =>
                      run({'type': 'variable.remove', 'name': v['name']}),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

/// Join this piece with another, or cut the other out of it.
class _CombineCard extends StatefulWidget {
  const _CombineCard({
    required this.panel,
    required this.others,
    required this.run,
  });

  final String panel;
  final List<Map<String, dynamic>> others;
  final ValueChanged<Map<String, Object?>> run;

  @override
  State<_CombineCard> createState() => _CombineCardState();
}

class _CombineCardState extends State<_CombineCard> {
  String? _with;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final picked = widget.others.any((o) => o['id'] == _with) ? _with : null;
    void apply(String op) => widget.run({
      'type': 'panel.boolean',
      'panel': widget.panel,
      'with': picked,
      'op': op,
    });
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l.combine,
          style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.s),
        DropdownButtonFormField<String>(
          key: const Key('combine-with'),
          initialValue: picked,
          isExpanded: true,
          decoration: InputDecoration(labelText: l.combineWith, isDense: true),
          items: [
            for (final o in widget.others)
              DropdownMenuItem(
                value: o['id'] as String,
                child: Text(
                  o['name'] as String,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
          ],
          onChanged: (v) => setState(() => _with = v),
        ),
        const SizedBox(height: Space.s),
        Row(
          children: [
            Expanded(
              child: OutlinedButton(
                onPressed: picked == null ? null : () => apply('union'),
                child: Text(l.join),
              ),
            ),
            const SizedBox(width: Space.s),
            Expanded(
              child: OutlinedButton(
                onPressed: picked == null ? null : () => apply('difference'),
                child: Text(l.cutOut),
              ),
            ),
          ],
        ),
      ],
    );
  }
}
