// An object project's panel (docs/specs/36-objects.md): how firmly its
// closed parts are stuffed, and a way back to a garment worn on a body.

import 'dart:convert';

import 'package:material_ui/material_ui.dart';

import 'busy.dart';
import 'design/tokens.dart';
import 'i18n.dart';
import 'rust/api/engine.dart';

/// How firmly the object's closed parts are stuffed (0–1), as a slider
/// from none to firm; [onChangeEnd] gets the value let go.
class StuffingSlider extends StatelessWidget {
  const StuffingSlider({
    super.key,
    required this.value,
    required this.onChanged,
    this.onChangeEnd,
  });

  final double value;
  final ValueChanged<double> onChanged;
  final ValueChanged<double>? onChangeEnd;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final small = ts(fontSize: 12, color: p.textMuted);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          l.stuffing,
          style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
        ),
        Slider(
          key: const Key('stuffing'),
          value: value,
          divisions: 10,
          label: '${(value * 100).round()} %',
          onChanged: onChanged,
          onChangeEnd: onChangeEnd,
        ),
        Row(
          children: [
            Text(l.stuffingNone, style: small),
            const Spacer(),
            Text(l.stuffingFirm, style: small),
          ],
        ),
        const SizedBox(height: Space.xs),
        Text(l.stuffingHint, style: ts(fontSize: 12, color: p.textMuted)),
      ],
    );
  }
}

/// Open the panel of an object project whose stuffing is [stuffing];
/// [onChanged] after each change.
Future<void> showObjectPanel(
  BuildContext context,
  Engine engine,
  Map<String, dynamic> standardBody,
  double stuffing,
  VoidCallback onChanged,
) {
  return showDialog<void>(
    context: context,
    builder: (_) => Dialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(kRadius * 1.4),
      ),
      child: SizedBox(
        width: 440,
        child: _ObjectPanel(
          engine: engine,
          standardBody: standardBody,
          stuffing: stuffing,
          onChanged: onChanged,
        ),
      ),
    ),
  );
}

class _ObjectPanel extends StatefulWidget {
  const _ObjectPanel({
    required this.engine,
    required this.standardBody,
    required this.stuffing,
    required this.onChanged,
  });

  final Engine engine;
  final Map<String, dynamic> standardBody;
  final double stuffing;
  final VoidCallback onChanged;

  @override
  State<_ObjectPanel> createState() => _ObjectPanelState();
}

class _ObjectPanelState extends State<_ObjectPanel> {
  late double _stuffing = widget.stuffing;
  String? _error;

  void _setStuffing(double v) {
    try {
      widget.engine.run(
        stepJson: jsonEncode({'type': 'object.set', 'stuffing': v}),
      );
      setState(() => _error = null);
      widget.onChanged();
    } on ApiError catch (e) {
      setState(() => _error = context.error(e));
    }
  }

  /// A garment again, on the standard body (fitted: seconds).
  Future<void> _wear() async {
    final engine = widget.engine;
    final l = context.l;
    try {
      await runBusy(
        context,
        engine,
        l.busyBodyTitle,
        () => engine.runLong(
          stepJson: jsonEncode({
            'type': 'batch',
            'label': 'object.clear',
            'commands': [
              {'type': 'object.clear'},
              {
                'type': 'avatar.set',
                'body': widget.standardBody['body'],
                'preset': widget.standardBody['name'],
              },
            ],
          }),
        ),
        step: l.busyBodyFit,
      );
    } on ApiError catch (e) {
      if (mounted) setState(() => _error = context.error(e));
      return;
    }
    widget.onChanged();
    if (mounted) Navigator.pop(context);
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return Padding(
      padding: const EdgeInsets.all(Space.xl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.view_in_ar_outlined, color: p.accent),
              const SizedBox(width: Space.s),
              Text(
                l.objectTitle,
                style: ts(fontSize: 20, fontWeight: FontWeight.w700),
              ),
              const Spacer(),
              IconButton(
                onPressed: () => Navigator.pop(context),
                icon: Icon(Icons.close_rounded, color: p.textMuted),
              ),
            ],
          ),
          Text(
            l.objectSubtitle,
            style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
          ),
          const SizedBox(height: Space.l),
          StuffingSlider(
            value: _stuffing,
            onChanged: (v) => setState(() => _stuffing = v),
            onChangeEnd: _setStuffing,
          ),
          if (_error != null) ...[
            const SizedBox(height: Space.m),
            Text(_error!, style: ts(fontSize: 13, color: p.danger)),
          ],
          const SizedBox(height: Space.l),
          Row(
            children: [
              TextButton.icon(
                key: const Key('objectWear'),
                onPressed: _wear,
                icon: const Icon(Icons.accessibility_new_rounded, size: 18),
                label: Text(l.objectWear),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
