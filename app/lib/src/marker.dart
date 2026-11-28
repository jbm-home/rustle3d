// The marker: a garment's pieces, in the sizes picked, laid out on the
// fabric, with the length to buy, the share of the fabric used and the
// cost (docs/specs/23-production.md).

part of 'shell.dart';

/// Colours of a marker's sizes after the first, as the graded nest prints
/// them (r3d-io `SIZE_COLORS`).
const _sizeColors = [
  Color(0xFFD62728),
  Color(0xFF1F77B4),
  Color(0xFF2CA02C),
  Color(0xFF9467BD),
  Color(0xFFFF7F0E),
  Color(0xFF8C564B),
  Color(0xFFE377C2),
  Color(0xFF17BECF),
];

/// The colour of the marker's size number [rank] (0: the first).
Color _sizeColor(Palette p, int rank) =>
    rank == 0 ? p.accent : _sizeColors[(rank - 1) % _sizeColors.length];

/// Pick the garment, its sizes and the fabric width; see the pieces laid
/// out with the length, efficiency, yardage and cost; save it as SVG or
/// PDF.
class _MarkerDialog extends StatefulWidget {
  const _MarkerDialog({
    required this.engine,
    required this.summary,
    required this.indicators,
  });

  final Engine engine;

  /// `project.summary`: the garments and the size range.
  final Map<String, dynamic> summary;

  /// `production.indicators`: each garment's fabric details.
  final Map<String, dynamic>? indicators;

  @override
  State<_MarkerDialog> createState() => _MarkerDialogState();
}

class _MarkerDialogState extends State<_MarkerDialog> {
  late String? _garment;
  final Set<String> _sizes = {};
  final _width = TextEditingController();

  /// The fabric width (cm) last put in the field, kept exact while the
  /// field shows it rounded to inches.
  double _widthCm = 150;
  Map<String, dynamic>? _marker;
  String? _error;
  String? _saved;
  bool _busy = false;

  List<Map<String, dynamic>> get _garments => [
    for (final g in (widget.summary['garments'] as List? ?? const []))
      g as Map<String, dynamic>,
  ];

  List<String> get _allSizes => [
    for (final s
        in ((widget.summary['grading'] as Map?)?['sizes'] as List?) ?? const [])
      s as String,
  ];

  /// The garment's fabric width (cm), if set.
  double? _fabricWidthCm(String? garment) {
    for (final g in (widget.indicators?['garments'] as List? ?? const [])) {
      if (g['id'] == garment) {
        final w = g['fabric_details']?['width_mm'] as num?;
        return w == null ? null : w / 10;
      }
    }
    return null;
  }

  @override
  void initState() {
    super.initState();
    _garment = _garments.isEmpty ? null : _garments.first['id'] as String;
    final shown = (widget.summary['grading'] as Map?)?['shown'] as String?;
    if (shown != null) _sizes.add(shown);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _showWidth(_fabricWidthCm(_garment) ?? 150);
      _layOut();
    });
  }

  @override
  void dispose() {
    _width.dispose();
    super.dispose();
  }

  void _showWidth(double cm) {
    _widthCm = cm;
    _width.text = context.fieldText(Quantity.cm, cm);
  }

  /// The width in the field (cm), exact if left as shown.
  double? get _typedWidthCm =>
      _width.text == context.fieldText(Quantity.cm, _widthCm)
      ? _widthCm
      : context.fieldValue(Quantity.cm, _width.text);

  String get _spec => jsonEncode({
    'garment': ?_garment,
    if (_allSizes.isNotEmpty)
      'sizes': [
        for (final s in _allSizes)
          if (_sizes.contains(s)) s,
      ],
    if (_typedWidthCm case final cm?) 'width_mm': cm * 10,
  });

  Future<void> _layOut() async {
    if (_allSizes.isNotEmpty && _sizes.isEmpty) return;
    setState(() {
      _busy = true;
      _saved = null;
    });
    try {
      final out = await widget.engine.marker(specJson: _spec);
      if (!mounted) return;
      setState(() {
        _marker = jsonDecode(out) as Map<String, dynamic>;
        _error = null;
      });
    } on ApiError catch (e) {
      if (!mounted) return;
      setState(() {
        _marker = null;
        _error = context.error(e);
      });
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _save(bool pdf) async {
    final l = context.l;
    final ext = pdf ? 'pdf' : 'svg';
    final name = (widget.summary['name'] as String?) ?? 'Rustle3D';
    final location = await getSaveLocation(
      suggestedName: '$name-marker.$ext',
      acceptedTypeGroups: [
        XTypeGroup(label: ext.toUpperCase(), extensions: [ext]),
      ],
    );
    if (location == null || !mounted) return;
    try {
      await runBusy(
        context,
        widget.engine,
        l.export,
        () => widget.engine.exportMarker(
          specJson: _spec,
          path: location.path,
          pdf: pdf,
        ),
      );
      if (mounted) {
        setState(
          () => _saved = l.exported(location.path.split(RegExp(r'[/\\]')).last),
        );
      }
    } on ApiError catch (e) {
      if (mounted) setState(() => _error = context.error(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final m = _marker;
    final garments = _garments;
    String num2(Object? v, int digits) =>
        v is num ? context.number(v, digits) : '—';
    return AlertDialog(
      title: Text(l.marker),
      content: SizedBox(
        width: 760,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              l.markerHint,
              style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
            ),
            const SizedBox(height: Space.m),
            if (garments.length > 1) ...[
              DropdownButtonFormField<String>(
                initialValue: _garment,
                decoration: InputDecoration(labelText: l.markerGarment),
                items: [
                  for (final g in garments)
                    DropdownMenuItem(
                      value: g['id'] as String,
                      child: Text(g['name'] as String),
                    ),
                ],
                onChanged: (g) {
                  setState(() => _garment = g);
                  if (_fabricWidthCm(g) case final cm?) _showWidth(cm);
                  _layOut();
                },
              ),
              const SizedBox(height: Space.m),
            ],
            if (_allSizes.isNotEmpty) ...[
              Text(l.markerSizes, style: ts(fontSize: 13, color: p.text)),
              const SizedBox(height: Space.xs),
              Wrap(
                spacing: Space.xs,
                runSpacing: Space.xs,
                children: [
                  for (final s in _allSizes)
                    FilterChip(
                      key: Key('marker-size-$s'),
                      label: Text(s),
                      selected: _sizes.contains(s),
                      onSelected: (on) {
                        setState(() => on ? _sizes.add(s) : _sizes.remove(s));
                        _layOut();
                      },
                    ),
                ],
              ),
              const SizedBox(height: Space.m),
            ],
            Row(
              children: [
                Text(l.markerWidth, style: ts(fontSize: 13, color: p.text)),
                const SizedBox(width: Space.s),
                SizedBox(
                  width: 96,
                  child: _NumberField(
                    key: const Key('marker-width'),
                    controller: _width,
                    unit: context.unitOf(Quantity.cm),
                    onSubmitted: _layOut,
                  ),
                ),
                const Spacer(),
                FilledButton.icon(
                  onPressed: _busy ? null : _layOut,
                  icon: const Icon(Icons.dashboard_outlined, size: 18),
                  label: Text(l.markerLayOut),
                ),
              ],
            ),
            const SizedBox(height: Space.m),
            Container(
              height: 240,
              decoration: BoxDecoration(
                border: Border.all(color: p.border),
                borderRadius: BorderRadius.circular(Tokens.radius(context)),
              ),
              clipBehavior: Clip.antiAlias,
              child: switch ((_busy, _error, m)) {
                (true, _, _) => const Center(
                  child: CircularProgressIndicator(),
                ),
                (_, final String e, _) => Center(
                  child: Padding(
                    padding: const EdgeInsets.all(Space.l),
                    child: Text(
                      e,
                      textAlign: TextAlign.center,
                      style: ts(fontSize: 13, color: p.danger),
                    ),
                  ),
                ),
                (_, _, final Map<String, dynamic> m) => _MarkerPreview(
                  marker: m,
                ),
                _ => const SizedBox.shrink(),
              },
            ),
            const SizedBox(height: Space.m),
            Row(
              children: [
                Expanded(
                  child: _Stat(
                    label: l.markerLength,
                    value: switch (m?['length_mm']) {
                      final num mm => context.measure(Quantity.m, mm / 1000, 2),
                      _ => '—',
                    },
                  ),
                ),
                const SizedBox(width: Space.s),
                Expanded(
                  child: _Stat(
                    label: l.markerEfficiency,
                    value: '${num2(m?['efficiency_pct'], 1)} %',
                  ),
                ),
                const SizedBox(width: Space.s),
                Expanded(
                  child: _Stat(
                    label: l.markerYardage,
                    value: switch (m?['yardage_m']) {
                      final num v => context.measure(Quantity.m, v, 2),
                      _ => '—',
                    },
                  ),
                ),
                const SizedBox(width: Space.s),
                Expanded(
                  child: _Stat(label: l.markerCost, value: num2(m?['cost'], 2)),
                ),
              ],
            ),
            if (m != null && m['cost'] == null) ...[
              const SizedBox(height: Space.s),
              Text(
                l.markerNoPrice,
                style: ts(fontSize: 12, height: 1.4, color: p.textMuted),
              ),
            ],
            if (_saved case final saved?) ...[
              const SizedBox(height: Space.s),
              Text(saved, style: ts(fontSize: 12, color: p.textMuted)),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l.close),
        ),
        OutlinedButton(
          onPressed: m == null || _busy ? null : () => _save(false),
          child: Text(l.markerSaveSvg),
        ),
        FilledButton(
          onPressed: m == null || _busy ? null : () => _save(true),
          child: Text(l.markerSavePdf),
        ),
      ],
    );
  }
}

/// The laid-out marker to scale, the fabric's width filling the height;
/// scrolls along a long marker.
class _MarkerPreview extends StatelessWidget {
  const _MarkerPreview({required this.marker});

  final Map<String, dynamic> marker;

  static const _pad = 12.0;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return LayoutBuilder(
      builder: (context, box) {
        final width = (marker['width_mm'] as num).toDouble();
        final length = (marker['length_mm'] as num).toDouble();
        final scale = (box.maxHeight - 2 * _pad) / width;
        final drawn = max(box.maxWidth, length * scale + 2 * _pad);
        return SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: CustomPaint(
            size: Size(drawn, box.maxHeight),
            painter: _MarkerPainter(
              marker: marker,
              scale: scale,
              pad: _pad,
              fabric: p.surfaceMuted,
              edge: p.border,
              colorOf: (rank) => _sizeColor(p, rank),
            ),
          ),
        );
      },
    );
  }
}

class _MarkerPainter extends CustomPainter {
  _MarkerPainter({
    required this.marker,
    required this.scale,
    required this.pad,
    required this.fabric,
    required this.edge,
    required this.colorOf,
  });

  final Map<String, dynamic> marker;
  final double scale;
  final double pad;
  final Color fabric;
  final Color edge;
  final Color Function(int rank) colorOf;

  @override
  void paint(Canvas canvas, Size size) {
    final width = (marker['width_mm'] as num).toDouble();
    final length = (marker['length_mm'] as num).toDouble();
    // Marker mm (y up from the bottom selvedge) to the canvas.
    Offset at(List<dynamic> q) => Offset(
      pad + (q[0] as num) * scale,
      pad + (width - (q[1] as num)) * scale,
    );
    final cloth = Rect.fromLTWH(pad, pad, length * scale, width * scale);
    canvas.drawRect(cloth, Paint()..color = fabric);
    canvas.drawRect(
      cloth,
      Paint()
        ..color = edge
        ..style = PaintingStyle.stroke,
    );
    for (final piece in marker['pieces'] as List) {
      final cut = piece['cut'] as List;
      if (cut.isEmpty) continue;
      final path = Path()..moveTo(at(cut.first).dx, at(cut.first).dy);
      for (final q in cut.skip(1)) {
        path.lineTo(at(q).dx, at(q).dy);
      }
      path.close();
      final color = colorOf(piece['size_rank'] as int? ?? 0);
      canvas.drawPath(path, Paint()..color = color.withValues(alpha: 0.16));
      canvas.drawPath(
        path,
        Paint()
          ..color = color
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1,
      );
    }
  }

  @override
  bool shouldRepaint(_MarkerPainter old) =>
      old.marker != marker || old.scale != scale || old.fabric != fabric;
}
