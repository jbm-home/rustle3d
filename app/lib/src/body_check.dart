// The imported body's check (spec 27): how its file is read (units, up
// axis, turn), what may go wrong when draping, and its arrangement points
// to correct.

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'busy.dart';
import 'design/tokens.dart';
import 'i18n.dart';
import 'region.dart';
import 'rust/api/engine.dart';

Future<void> showBodyCheck(
  BuildContext context,
  Engine engine,
  VoidCallback onChanged,
) {
  return showDialog<void>(
    context: context,
    builder: (_) => Dialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(kRadius * 1.4),
      ),
      child: SizedBox(
        width: 620,
        child: _BodyCheck(engine: engine, onChanged: onChanged),
      ),
    ),
  );
}

/// An arrangement point's name (`left_elbow`) in the UI language.
String pointName(L l, String name) => switch (name) {
  'chest' => l.pointChest,
  'waist' => l.pointWaist,
  'hips' => l.pointHips,
  'crotch' => l.pointCrotch,
  'left_shoulder' => l.pointLeftShoulder,
  'left_elbow' => l.pointLeftElbow,
  'left_wrist' => l.pointLeftWrist,
  'right_shoulder' => l.pointRightShoulder,
  'right_elbow' => l.pointRightElbow,
  'right_wrist' => l.pointRightWrist,
  'left_thigh' => l.pointLeftThigh,
  'left_shin' => l.pointLeftShin,
  'right_thigh' => l.pointRightThigh,
  'right_shin' => l.pointRightShin,
  _ => name,
};

/// The worst severity among a check's items (`error`, `warning`), or
/// `null` when nothing needs a look.
String? checkConcern(Map<String, dynamic>? check) {
  final items = (check?['items'] as List?) ?? const [];
  if (items.any((i) => i['severity'] == 'error')) return 'error';
  if (items.any((i) => i['severity'] == 'warning')) return 'warning';
  return null;
}

/// One item of the check in the UI language.
String checkText(BuildContext context, Map<String, dynamic> item) {
  final l = context.l;
  final args = (item['args'] as Map?) ?? const {};
  String a(String k) => '${args[k] ?? ''}';
  return switch (item['code']) {
    'file.unavailable' => l.checkFileUnavailable(a('file')),
    'mesh.open' => l.checkMeshOpen(a('holes'), a('edges')),
    'mesh.non_manifold' => l.checkMeshNonManifold(a('edges')),
    'mesh.pieces' => l.checkMeshPieces(a('pieces')),
    'mesh.heavy' => l.checkMeshHeavy(a('triangles')),
    'mesh.coarse' => l.checkMeshCoarse(a('triangles')),
    'mesh.reoriented' => l.checkMeshReoriented(a('triangles')),
    'mesh.inside_out' => l.checkMeshInsideOut,
    'mesh.degenerate' => l.checkMeshDegenerate(a('triangles')),
    'scale.height' => l.checkScaleHeight(a('height_m')),
    'orientation.up' => l.checkOrientationUp,
    'orientation.facing' => l.checkOrientationFacing,
    'points.outside' => l.checkPointsOutside(pointName(l, a('point'))),
    'points.side' => l.checkPointsSide(pointName(l, a('point'))),
    'measurements.clamped' => l.checkMeasurementsClamped(a('measurement')),
    'motion.rig' =>
      a('source') == 'file' ? l.checkMotionFileSkeleton : l.checkMotionPoints,
    final code => '$code',
  };
}

const _units = [1.0, 0.01, 0.001, 0.0254];
const _axes = ['y', 'z', 'x', '-y', '-z', '-x'];

class _BodyCheck extends StatefulWidget {
  const _BodyCheck({required this.engine, required this.onChanged});

  final Engine engine;
  final VoidCallback onChanged;

  @override
  State<_BodyCheck> createState() => _BodyCheckState();
}

class _BodyCheckState extends State<_BodyCheck> {
  Map<String, dynamic>? _check;
  String? _error;
  final _turn = TextEditingController();
  final _points = <String, List<TextEditingController>>{};

  /// Each point's place (mm), as checked.
  final _pointsMm = <String, List<double>>{};

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _showPoints();
  }

  String _shown(double mm) => context.fieldText(Quantity.cm, mm / 10, 1);

  /// The points' fields, in cm or inches.
  void _showPoints() {
    for (final MapEntry(key: name, value: at) in _pointsMm.entries) {
      final row = _points.putIfAbsent(
        name,
        () => [for (var i = 0; i < 3; i++) TextEditingController()],
      );
      for (final (i, mm) in at.indexed) {
        row[i].text = _shown(mm);
      }
    }
  }

  @override
  void dispose() {
    _turn.dispose();
    for (final row in _points.values) {
      for (final c in row) {
        c.dispose();
      }
    }
    super.dispose();
  }

  void _load() {
    try {
      final check = jsonDecode(
        widget.engine.run(stepJson: '{"type":"avatar.check"}'),
      ) as Map<String, dynamic>;
      _check = check;
      _turn.text = (check['turn_deg'] as num).toStringAsFixed(0);
      for (final p in (check['points'] as List).cast<Map>()) {
        _pointsMm[p['point'] as String] = [
          for (final v in (p['at_mm'] as List).take(3)) (v as num).toDouble(),
        ];
      }
    } on ApiError catch (e) {
      _error = context.error(e);
    }
  }

  /// Run a step that guesses the points again or rebuilds the body
  /// (a second for a large scan), then show the check again.
  Future<void> _step(Map<String, Object?> step) async {
    try {
      await runBusy(
        context,
        widget.engine,
        context.l.bodyCheckTitle,
        () => widget.engine.runLong(stepJson: jsonEncode(step)),
      );
      if (!mounted) return;
      widget.engine.release();
      setState(() {
        _error = null;
        _load();
        _showPoints();
      });
      widget.onChanged();
    } on ApiError catch (e) {
      if (mounted) setState(() => _error = context.error(e));
    }
  }

  void _setPoint(String name) {
    final row = _points[name];
    final was = _pointsMm[name];
    if (row == null || was == null) return;
    // A coordinate left as shown stays exact (not rounded to inches).
    final mm = [
      for (final (i, c) in row.indexed)
        if (i < was.length && c.text == _shown(was[i]))
          was[i]
        else if (context.fieldValue(Quantity.cm, c.text) case final cm?)
          cm * 10,
    ];
    if (mm.length != row.length) return;
    _step({'type': 'avatar.set_point', 'point': name, 'at_mm': mm});
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final check = _check;
    final items = ((check?['items'] as List?) ?? const [])
        .cast<Map<String, dynamic>>();
    final ready = check?['ready'] == true;
    final scale = (check?['scale'] as num?)?.toDouble() ?? 1;
    final units = _units.contains(scale) ? _units : [..._units, scale];
    return ConstrainedBox(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.sizeOf(context).height * 0.85,
      ),
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(Space.xl),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.fact_check_outlined, color: p.accent),
                const SizedBox(width: Space.s),
                Text(
                  l.bodyCheckTitle,
                  style: ts(fontSize: 20, fontWeight: FontWeight.w700),
                ),
                const Spacer(),
                IconButton(
                  onPressed: () => Navigator.pop(context),
                  icon: Icon(Icons.close_rounded, color: p.textMuted),
                ),
              ],
            ),
            if (check != null) ...[
              Text(
                l.bodyCheckSummary(
                  check['file'] as String,
                  context.number(check['triangles'] as num),
                  context.number(check['height_m'] as num, 2),
                ),
                style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
              ),
              const SizedBox(height: Space.s),
              Text(
                ready ? l.bodyCheckReady : l.bodyCheckNotReady,
                key: const Key('bodyCheckReady'),
                style: ts(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                  color: ready ? p.accent : p.danger,
                ),
              ),
              const SizedBox(height: Space.l),
              Text(
                l.bodyReading,
                style: ts(fontSize: 14, fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: Space.s),
              Wrap(
                spacing: Space.m,
                runSpacing: Space.m,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  SizedBox(
                    width: 190,
                    child: DropdownButtonFormField<double>(
                      key: const Key('bodyUnits'),
                      initialValue: scale,
                      decoration: InputDecoration(
                        labelText: l.bodyUnits,
                        border: const OutlineInputBorder(),
                      ),
                      items: [
                        for (final u in units)
                          DropdownMenuItem(
                            value: u,
                            child: Text(switch (u) {
                              1.0 => l.unitMetres,
                              0.01 => l.unitCentimetres,
                              0.001 => l.unitMillimetres,
                              0.0254 => l.unitInches,
                              _ => l.unitOther('$u'),
                            }),
                          ),
                      ],
                      onChanged: (u) {
                        if (u != null && u != scale) {
                          _step({'type': 'avatar.orient', 'scale': u});
                        }
                      },
                    ),
                  ),
                  SizedBox(
                    width: 130,
                    child: DropdownButtonFormField<String>(
                      key: const Key('bodyUp'),
                      initialValue: check['up'] as String,
                      decoration: InputDecoration(
                        labelText: l.bodyUp,
                        border: const OutlineInputBorder(),
                      ),
                      items: [
                        for (final a in _axes)
                          DropdownMenuItem(
                            value: a,
                            child: Text(a.toUpperCase()),
                          ),
                      ],
                      onChanged: (a) {
                        if (a != null && a != check['up']) {
                          _step({'type': 'avatar.orient', 'up': a});
                        }
                      },
                    ),
                  ),
                  SizedBox(
                    width: 130,
                    child: TextField(
                      key: const Key('bodyTurn'),
                      controller: _turn,
                      keyboardType: const TextInputType.numberWithOptions(
                        signed: true,
                        decimal: true,
                      ),
                      inputFormatters: [
                        FilteringTextInputFormatter.allow(RegExp(r'[0-9.,-]')),
                      ],
                      onSubmitted: (text) {
                        final deg = double.tryParse(text.replaceAll(',', '.'));
                        if (deg != null) {
                          _step({'type': 'avatar.orient', 'turn_deg': deg});
                        }
                      },
                      decoration: InputDecoration(
                        labelText: l.bodyTurn,
                        suffixText: '°',
                        border: const OutlineInputBorder(),
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.l),
              if (items.every((i) => i['severity'] == 'info'))
                Text(l.bodyCheckAllGood, style: ts(fontSize: 13)),
              for (final item in items)
                Padding(
                  padding: const EdgeInsets.only(bottom: Space.xs),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(
                        switch (item['severity']) {
                          'error' => Icons.error_outline_rounded,
                          'warning' => Icons.warning_amber_rounded,
                          _ => Icons.info_outline_rounded,
                        },
                        size: 18,
                        color: item['severity'] == 'info'
                            ? p.textMuted
                            : p.danger,
                      ),
                      const SizedBox(width: Space.s),
                      Expanded(
                        child: Text(
                          checkText(context, item),
                          style: ts(fontSize: 13, height: 1.4),
                        ),
                      ),
                    ],
                  ),
                ),
              const SizedBox(height: Space.l),
              Row(
                children: [
                  Text(
                    l.bodyPoints,
                    style: ts(fontSize: 14, fontWeight: FontWeight.w600),
                  ),
                  const Spacer(),
                  TextButton.icon(
                    key: const Key('bodyGuessAgain'),
                    onPressed: () => _step({'type': 'avatar.guess_points'}),
                    icon: const Icon(Icons.auto_fix_high_outlined, size: 18),
                    label: Text(l.bodyGuessAgain),
                  ),
                ],
              ),
              Text(
                l.bodyPointsHint(context.unitOf(Quantity.cm)),
                style: ts(fontSize: 12.5, color: p.textMuted, height: 1.4),
              ),
              const SizedBox(height: Space.s),
              for (final point in (check['points'] as List).cast<Map>())
                _PointRow(
                  name: point['point'] as String,
                  outside: (point['distance_mm'] as num) > 5,
                  fields: _points[point['point']] ?? const [],
                  onSubmit: () => _setPoint(point['point'] as String),
                ),
            ],
            if (_error != null) ...[
              const SizedBox(height: Space.m),
              Text(_error!, style: ts(fontSize: 13, color: p.danger)),
            ],
          ],
        ),
      ),
    );
  }
}

/// A point's name and its three coordinates (cm), to edit.
class _PointRow extends StatelessWidget {
  const _PointRow({
    required this.name,
    required this.outside,
    required this.fields,
    required this.onSubmit,
  });

  final String name;
  final bool outside;
  final List<TextEditingController> fields;
  final VoidCallback onSubmit;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          SizedBox(
            width: 170,
            child: Text(
              pointName(context.l, name),
              style: ts(fontSize: 13, color: outside ? p.danger : p.text),
            ),
          ),
          for (final c in fields) ...[
            SizedBox(
              width: 96,
              child: TextField(
                controller: c,
                keyboardType: const TextInputType.numberWithOptions(
                  signed: true,
                  decimal: true,
                ),
                onSubmitted: (_) => onSubmit(),
                style: ts(fontSize: 13, tabular: true),
                decoration: InputDecoration(
                  isDense: true,
                  suffixText: context.unitOf(Quantity.cm),
                  border: const OutlineInputBorder(),
                ),
              ),
            ),
            const SizedBox(width: Space.s),
          ],
        ],
      ),
    );
  }
}
