// The Body panel's "More measurements" (spec 29): neck, shoulders, arms,
// legs… grouped, each with a small figure of where the tape goes, its
// value typed or estimated, and a way back to the estimate.

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'region.dart';

/// The 3D body this far (cm) from a value it should take shows the gap.
const _gapCm = 1.0;

/// The groups, in the order `avatar.get` lists the measurements' groups.
const _groups = ['torso', 'arms', 'legs'];

String _groupLabel(L l, String group) => switch (group) {
  'arms' => l.bodyArms,
  'legs' => l.bodyLegs,
  _ => l.bodyTorso,
};

(String, String) _labels(L l, String name) => switch (name) {
  'neck' => (l.measNeck, l.measNeckHint),
  'shoulder_width' => (l.measShoulderWidth, l.measShoulderWidthHint),
  'shoulder_length' => (l.measShoulderLength, l.measShoulderLengthHint),
  'back_waist_length' => (l.measBackWaistLength, l.measBackWaistLengthHint),
  'front_waist_length' => (l.measFrontWaistLength, l.measFrontWaistLengthHint),
  'waist_height' => (l.measWaistHeight, l.measWaistHeightHint),
  'hip_height' => (l.measHipHeight, l.measHipHeightHint),
  'crotch_depth' => (l.measCrotchDepth, l.measCrotchDepthHint),
  'arm_length' => (l.measArmLength, l.measArmLengthHint),
  'upper_arm' => (l.measUpperArm, l.measUpperArmHint),
  'wrist' => (l.measWrist, l.measWristHint),
  'inside_leg' => (l.measInsideLeg, l.measInsideLegHint),
  'thigh' => (l.measThigh, l.measThighHint),
  'knee' => (l.measKnee, l.measKneeHint),
  'ankle' => (l.measAnkle, l.measAnkleHint),
  _ => (name, ''),
};

/// The measurements besides the four, as `avatar.get` lists them; [onSet]
/// types one (cm) or, with null, brings its estimate back.
class MoreMeasurements extends StatefulWidget {
  const MoreMeasurements({
    super.key,
    required this.measures,
    required this.onSet,
  });

  final List<Map<String, dynamic>> measures;
  final Future<void> Function(String name, double? cm) onSet;

  @override
  State<MoreMeasurements> createState() => _MoreMeasurementsState();
}

class _MoreMeasurementsState extends State<MoreMeasurements> {
  bool _open = false;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        InkWell(
          key: const Key('moreMeasurements'),
          borderRadius: BorderRadius.circular(kRadius * 0.75),
          onTap: () => setState(() => _open = !_open),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: Space.s),
            child: Row(
              children: [
                Icon(
                  _open ? Icons.expand_less_rounded : Icons.expand_more_rounded,
                  color: p.textMuted,
                ),
                const SizedBox(width: Space.s),
                Text(
                  l.bodyMore,
                  style: ts(fontSize: 14, fontWeight: FontWeight.w600),
                ),
              ],
            ),
          ),
        ),
        if (_open) ...[
          Text(
            l.bodyMoreHint,
            style: ts(fontSize: 12.5, color: p.textMuted, height: 1.4),
          ),
          for (final group in _groups) ...[
            const SizedBox(height: Space.m),
            Text(
              _groupLabel(l, group).toUpperCase(),
              style: ts(
                fontSize: 11,
                fontWeight: FontWeight.w700,
                color: p.textMuted,
                letterSpacing: 0.8,
              ),
            ),
            for (final m in widget.measures.where((m) => m['group'] == group))
              _Row(
                key: ValueKey(m['name']),
                measure: m,
                onSet: (cm) => widget.onSet(m['name'] as String, cm),
              ),
          ],
        ],
      ],
    );
  }
}

class _Row extends StatefulWidget {
  const _Row({super.key, required this.measure, required this.onSet});

  final Map<String, dynamic> measure;
  final Future<void> Function(double? cm) onSet;

  @override
  State<_Row> createState() => _RowState();
}

class _RowState extends State<_Row> {
  final _field = TextEditingController();
  final _focus = FocusNode();

  bool get _estimated => widget.measure['estimated'] == true;
  double get _cm => (widget.measure['cm'] as num).toDouble();

  /// The region the field is written in.
  Region _region = const Region();

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
    _region = context.region;
    _show();
  }

  @override
  void didUpdateWidget(_Row old) {
    super.didUpdateWidget(old);
    if (!_focus.hasFocus) _show();
  }

  @override
  void dispose() {
    _field.dispose();
    _focus.dispose();
    super.dispose();
  }

  /// A typed value in the field; an estimate only as its hint.
  void _show() =>
      _field.text = _estimated ? '' : cmText(_region, _cm, digits: 1);

  void _submit() {
    final text = _field.text.trim();
    if (text.isEmpty) {
      if (!_estimated) widget.onSet(null);
      return;
    }
    final v = cmOf(_region, text);
    if (v == null || (!_estimated && text == cmText(_region, _cm, digits: 1))) {
      _show();
      return;
    }
    widget.onSet(v);
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final name = widget.measure['name'] as String;
    final (label, hint) = _labels(l, name);
    final reached = (widget.measure['reached_cm'] as num?)?.toDouble();
    // A typed value the 3D body does not reach.
    final gap =
        !_estimated && reached != null && (reached - _cm).abs() > _gapCm;
    return Padding(
      padding: const EdgeInsets.only(top: Space.s),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          SizedBox(
            width: 28,
            height: 56,
            child: CustomPaint(
              painter: _TapeFigure(
                name: name,
                body: p.textMuted.withValues(alpha: 0.45),
                tape: p.accent,
              ),
            ),
          ),
          const SizedBox(width: Space.m),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(label, style: ts(fontSize: 13.5)),
                Text(
                  hint,
                  style: ts(fontSize: 11.5, color: p.textMuted, height: 1.3),
                ),
                if (gap)
                  Text(
                    l.measAvatarReaches(
                      '${context.number(_region.fromCm(reached), 1)} '
                      '${_region.cmUnit(l)}',
                    ),
                    style: ts(fontSize: 11.5, color: p.danger, height: 1.3),
                  ),
              ],
            ),
          ),
          const SizedBox(width: Space.s),
          SizedBox(
            width: 104,
            child: TextField(
              key: Key('measure_$name'),
              controller: _field,
              focusNode: _focus,
              keyboardType: const TextInputType.numberWithOptions(
                decimal: true,
              ),
              inputFormatters: [
                FilteringTextInputFormatter.allow(RegExp(r'[0-9.,]')),
              ],
              onSubmitted: (_) => _submit(),
              style: ts(fontSize: 13.5, tabular: true),
              decoration: InputDecoration(
                isDense: true,
                hintText: context.number(_region.fromCm(_cm), 1),
                helperText: _estimated ? l.measEstimated : null,
                helperStyle: ts(fontSize: 10.5, color: p.textMuted),
                suffixText: _region.cmUnit(l),
                border: const OutlineInputBorder(),
              ),
            ),
          ),
          SizedBox(
            width: 36,
            child: _estimated
                ? null
                : IconButton(
                    key: Key('reset_$name'),
                    tooltip: l.measReset,
                    onPressed: () => widget.onSet(null),
                    icon: Icon(
                      Icons.restart_alt_rounded,
                      size: 18,
                      color: p.textMuted,
                    ),
                  ),
          ),
        ],
      ),
    );
  }
}

/// A small standing figure, seen from the front, with the tape drawn
/// where measurement [name] is taken. Coordinates are fractions of the
/// box (x across, y down).
class _TapeFigure extends CustomPainter {
  _TapeFigure({required this.name, required this.body, required this.tape});

  final String name;
  final Color body;
  final Color tape;

  static const _head = Offset(0.5, 0.075);
  static const _neck = Offset(0.5, 0.17);
  static const _shoulderL = Offset(0.27, 0.21);
  static const _shoulderR = Offset(0.73, 0.21);
  static const _elbowL = Offset(0.18, 0.39);
  static const _elbowR = Offset(0.82, 0.39);
  static const _wristL = Offset(0.14, 0.53);
  static const _wristR = Offset(0.86, 0.53);
  static const _waistY = 0.41;
  static const _hipY = 0.51;
  static const _crotch = Offset(0.5, 0.54);
  static const _kneeL = Offset(0.4, 0.75);
  static const _kneeR = Offset(0.6, 0.75);
  static const _ankleL = Offset(0.4, 0.95);
  static const _ankleR = Offset(0.6, 0.95);

  @override
  void paint(Canvas canvas, Size size) {
    Offset at(Offset f) => Offset(f.dx * size.width, f.dy * size.height);
    final limb = Paint()
      ..color = body
      ..strokeWidth = size.width * 0.1
      ..strokeCap = StrokeCap.round
      ..style = PaintingStyle.stroke;
    final fill = Paint()..color = body;
    canvas.drawCircle(at(_head), size.width * 0.12, fill);
    canvas.drawLine(at(_head), at(_neck), limb);
    final torso = Path()
      ..moveTo(at(_shoulderL).dx, at(_shoulderL).dy)
      ..lineTo(at(_shoulderR).dx, at(_shoulderR).dy)
      ..lineTo(0.64 * size.width, _waistY * size.height)
      ..lineTo(0.69 * size.width, _hipY * size.height)
      ..lineTo(at(_crotch).dx, at(_crotch).dy)
      ..lineTo(0.31 * size.width, _hipY * size.height)
      ..lineTo(0.36 * size.width, _waistY * size.height)
      ..close();
    canvas.drawPath(torso, fill);
    for (final (a, b, c) in [
      (_shoulderL, _elbowL, _wristL),
      (_shoulderR, _elbowR, _wristR),
      (const Offset(0.4, 0.52), _kneeL, _ankleL),
      (const Offset(0.6, 0.52), _kneeR, _ankleR),
    ]) {
      canvas.drawLine(at(a), at(b), limb);
      canvas.drawLine(at(b), at(c), limb);
    }

    final line = Paint()
      ..color = tape
      ..strokeWidth = 1.6
      ..strokeCap = StrokeCap.round
      ..style = PaintingStyle.stroke;
    void path(List<Offset> points) {
      final p = Path()..moveTo(at(points.first).dx, at(points.first).dy);
      for (final q in points.skip(1)) {
        p.lineTo(at(q).dx, at(q).dy);
      }
      canvas.drawPath(p, line);
    }

    // A girth: a flat loop around the body at `centre`, `half` wide.
    void around(Offset centre, double half) => canvas.drawOval(
      Rect.fromCenter(
        center: at(centre),
        width: 2 * half * size.width,
        height: size.height * 0.035,
      ),
      line,
    );
    Offset mid(Offset a, Offset b, double t) => Offset.lerp(a, b, t)!;
    switch (name) {
      case 'neck':
        around(const Offset(0.5, 0.165), 0.09);
      case 'shoulder_width':
        path([_shoulderL, _shoulderR]);
      case 'shoulder_length':
        path([const Offset(0.44, 0.19), _shoulderL]);
      case 'back_waist_length':
        path([const Offset(0.5, 0.18), const Offset(0.5, _waistY)]);
      case 'front_waist_length':
        path([
          const Offset(0.44, 0.19),
          const Offset(0.4, 0.29),
          const Offset(0.4, _waistY),
        ]);
      case 'waist_height':
        path([const Offset(0.94, _waistY), const Offset(0.94, 0.98)]);
      case 'hip_height':
        path([const Offset(0.94, _hipY), const Offset(0.94, 0.98)]);
      case 'crotch_depth':
        path([const Offset(0.94, _waistY), const Offset(0.94, 0.54)]);
      case 'arm_length':
        path([_shoulderL, _elbowL, _wristL]);
      case 'upper_arm':
        around(mid(_shoulderL, _elbowL, 0.45), 0.08);
      case 'wrist':
        around(mid(_elbowL, _wristL, 0.9), 0.06);
      case 'inside_leg':
        path([const Offset(0.47, 0.56), const Offset(0.44, 0.97)]);
      case 'thigh':
        around(const Offset(0.4, 0.58), 0.1);
      case 'knee':
        around(_kneeL, 0.07);
      case 'ankle':
        around(mid(_kneeL, _ankleL, 0.88), 0.06);
    }
  }

  @override
  bool shouldRepaint(_TapeFigure old) =>
      old.name != name || old.body != body || old.tape != tape;
}
