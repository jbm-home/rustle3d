// The "Body" panel: choose a preset, enter measurements, fit the avatar,
// and manage the user's own presets.

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'body_picker.dart' show nearestSizeText;
import 'busy.dart';
import 'design/tokens.dart';
import 'i18n.dart';
import 'measurements.dart';
import 'rust/api/engine.dart';
import 'rust/api/library.dart';

/// Ranges accepted by the `avatar.set` command (cm).
const _ranges = {
  'height_cm': (50.0, 220.0),
  'bust_cm': (35.0, 170.0),
  'waist_cm': (33.0, 170.0),
  'hips_cm': (35.0, 180.0),
};

/// A child's age accepted by `avatar.set` (years).
const _ages = (0.0, 18.0);

String _label(L l, String key) => switch (key) {
  'height_cm' => l.measHeight,
  'bust_cm' => l.measBust,
  'waist_cm' => l.sizeWaist,
  'hips_cm' => l.sizeHips,
  _ => key,
};

Future<void> showBodyPanel(
  BuildContext context,
  Engine engine,
  String libraryDir,
  VoidCallback onChanged,
) {
  return showDialog<void>(
    context: context,
    builder: (_) => Dialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(kRadius * 1.4),
      ),
      child: SizedBox(
        width: 520,
        child: _BodyPanel(
          engine: engine,
          libraryDir: libraryDir,
          onChanged: onChanged,
        ),
      ),
    ),
  );
}

class _BodyPanel extends StatefulWidget {
  const _BodyPanel({
    required this.engine,
    required this.libraryDir,
    required this.onChanged,
  });

  final Engine engine;
  final String libraryDir;
  final VoidCallback onChanged;

  @override
  State<_BodyPanel> createState() => _BodyPanelState();
}

class _BodyPanelState extends State<_BodyPanel> {
  List<Map<String, dynamic>> _presets = const [];
  String? _preset;
  double _masculinity = 0;
  final _fields = {for (final k in _ranges.keys) k: TextEditingController()};

  /// A child's body (`age_years` set), and its age.
  bool _child = false;
  final _age = TextEditingController();
  Map<String, dynamic>? _achieved;
  List<Map<String, dynamic>> _measures = const [];
  String? _error;
  bool _naming = false;
  final _name = TextEditingController();

  @override
  void initState() {
    super.initState();
    _loadPresets();
    final current = jsonDecode(
      widget.engine.run(stepJson: '{"type":"avatar.get"}'),
    );
    if (current is Map<String, dynamic>) {
      _setBody(current['body'] as Map<String, dynamic>);
      _preset = current['preset'] as String?;
      _show(current);
    }
  }

  /// What `avatar.get` says the body reaches, and the other measurements.
  void _show(Map<String, dynamic> got) {
    _achieved = got['achieved'] as Map<String, dynamic>?;
    _measures = ((got['measures'] as List?) ?? const [])
        .cast<Map<String, dynamic>>();
  }

  /// Type one of the other measurements (cm), or with null bring its
  /// estimate back; the body is fitted again (seconds, off the UI
  /// thread).
  Future<void> _setMeasure(String name, double? cm) async {
    final engine = widget.engine;
    try {
      final got = await runBusy(
        context,
        engine,
        context.l.busyBodyTitle,
        () async {
          await engine.runLong(
            stepJson: jsonEncode({
              'type': 'avatar.set_measures',
              'measures': {name: cm},
            }),
          );
          return jsonDecode(
            await engine.runLong(stepJson: '{"type":"avatar.get"}'),
          );
        },
      );
      if (!mounted) return;
      setState(() {
        _error = null;
        _show(got as Map<String, dynamic>);
      });
      widget.onChanged();
    } on ApiError catch (e) {
      if (mounted) setState(() => _error = context.error(e));
    }
  }

  @override
  void dispose() {
    for (final c in _fields.values) {
      c.dispose();
    }
    _name.dispose();
    _age.dispose();
    super.dispose();
  }

  void _loadPresets() {
    _presets = (jsonDecode(bodyPresets(libraryDir: widget.libraryDir)) as List)
        .cast<Map<String, dynamic>>();
  }

  void _setBody(Map<String, dynamic> body) {
    _masculinity = (body['masculinity'] as num).toDouble();
    for (final e in _fields.entries) {
      e.value.text = (body[e.key] as num).toStringAsFixed(0);
    }
    final age = body['age_years'] as num?;
    _child = age != null;
    if (age != null) {
      _age.text = age % 1 == 0 ? age.toStringAsFixed(0) : '$age';
    }
  }

  Map<String, Object>? _readBody() {
    final body = <String, Object>{'masculinity': _masculinity};
    for (final e in _fields.entries) {
      final v = double.tryParse(e.value.text.replaceAll(',', '.'));
      final (min, max) = _ranges[e.key]!;
      if (v == null || v < min || v > max) {
        setState(
          () => _error = context.l.measBetween(
            _label(context.l, e.key),
            min.round(),
            max.round(),
          ),
        );
        return null;
      }
      body[e.key] = v;
    }
    if (_child) {
      final v = double.tryParse(_age.text.replaceAll(',', '.'));
      final (min, max) = _ages;
      if (v == null || v < min || v > max) {
        setState(
          () => _error = context.l.measBetween(
            context.l.bodyAge,
            min.round(),
            max.round(),
          ),
        );
        return null;
      }
      body['age_years'] = v;
    }
    return body;
  }

  /// Fitting the body to new measurements takes seconds: off the UI
  /// thread, with a card saying so.
  Future<void> _apply() async {
    final body = _readBody();
    if (body == null) return;
    final engine = widget.engine;
    try {
      final got = await runBusy(
        context,
        engine,
        context.l.busyBodyTitle,
        () async {
          await engine.runLong(
            stepJson: jsonEncode({
              'type': 'avatar.set',
              'body': body,
              'preset': ?_preset,
            }),
          );
          return jsonDecode(
            await engine.runLong(stepJson: '{"type":"avatar.get"}'),
          );
        },
      );
      if (!mounted) return;
      setState(() {
        _error = null;
        _show(got as Map<String, dynamic>);
      });
      widget.onChanged();
    } on ApiError catch (e) {
      if (mounted) setState(() => _error = context.error(e));
    }
  }

  void _choosePreset(String? name) {
    final preset = _presets.where((p) => p['name'] == name).firstOrNull;
    if (preset == null) return;
    setState(() {
      _preset = name;
      _setBody(preset['body'] as Map<String, dynamic>);
    });
    _apply();
  }

  void _savePreset() {
    final body = _readBody();
    final name = _name.text.trim();
    if (body == null || name.isEmpty) return;
    try {
      saveBodyPreset(
        libraryDir: widget.libraryDir,
        name: name,
        bodyJson: jsonEncode(body),
      );
      setState(() {
        _naming = false;
        _preset = name;
        _loadPresets();
      });
      _apply();
    } on ApiError catch (e) {
      setState(() => _error = context.error(e));
    }
  }

  void _deletePreset() {
    final name = _preset;
    if (name == null) return;
    deleteBodyPreset(libraryDir: widget.libraryDir, name: name);
    setState(() {
      _preset = null;
      _loadPresets();
    });
  }

  /// The size the fields make on their chart ("40 (Woman)"), none while
  /// one is not a number.
  String? _nearestText(L l) {
    final body = <String, Object>{'masculinity': _masculinity};
    for (final e in _fields.entries) {
      final v = double.tryParse(e.value.text.replaceAll(',', '.'));
      if (v == null) return null;
      body[e.key] = v;
    }
    if (_child) {
      final age = double.tryParse(_age.text.replaceAll(',', '.'));
      if (age == null) return null;
      body['age_years'] = age;
    }
    return nearestSizeText(l, body);
  }

  bool get _isCustom =>
      _presets.any((p) => p['name'] == _preset && p['builtin'] == false);

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final builtin = _presets.where((x) => x['builtin'] == true);
    final mine = _presets.where((x) => x['builtin'] == false);
    // The other measurements make it taller than some screens.
    return SingleChildScrollView(
      padding: const EdgeInsets.all(Space.xl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.accessibility_new_rounded, color: p.accent),
              const SizedBox(width: Space.s),
              Text(
                l.body,
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
            l.bodySubtitle,
            style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
          ),
          const SizedBox(height: Space.l),
          DropdownButtonFormField<String>(
            initialValue: _presets.any((x) => x['name'] == _preset)
                ? _preset
                : null,
            decoration: InputDecoration(
              labelText: l.bodyPreset,
              border: const OutlineInputBorder(),
            ),
            items: [
              for (final x in builtin)
                DropdownMenuItem(
                  value: x['name'] as String,
                  child: Text(context.presetName(x)),
                ),
              for (final x in mine)
                DropdownMenuItem(
                  value: x['name'] as String,
                  child: Row(
                    children: [
                      Icon(
                        Icons.person_outline_rounded,
                        size: 16,
                        color: p.accent,
                      ),
                      const SizedBox(width: 6),
                      Text(x['name'] as String),
                    ],
                  ),
                ),
            ],
            onChanged: _choosePreset,
          ),
          const SizedBox(height: Space.l),
          Wrap(
            spacing: Space.m,
            runSpacing: Space.m,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SegmentedButton<bool>(
                segments: [
                  ButtonSegment(value: false, label: Text(l.bodyAdult)),
                  ButtonSegment(value: true, label: Text(l.bodyChild)),
                ],
                selected: {_child},
                onSelectionChanged: (s) => setState(() {
                  _child = s.first;
                  if (_child && _age.text.isEmpty) _age.text = '6';
                  _preset = null;
                }),
              ),
              SegmentedButton<double>(
                segments: [
                  ButtonSegment(
                    value: 0,
                    label: Text(_child ? l.bodyGirl : l.bodyFeminine),
                  ),
                  ButtonSegment(
                    value: 1,
                    label: Text(_child ? l.bodyBoy : l.bodyMasculine),
                  ),
                ],
                selected: {_masculinity < 0.5 ? 0 : 1},
                onSelectionChanged: (s) => setState(() {
                  _masculinity = s.first;
                  _preset = null;
                }),
              ),
              if (_child)
                SizedBox(
                  width: 120,
                  child: TextField(
                    controller: _age,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    inputFormatters: [
                      FilteringTextInputFormatter.allow(RegExp(r'[0-9.,]')),
                    ],
                    onChanged: (_) => setState(() => _preset = null),
                    onSubmitted: (_) => _apply(),
                    decoration: InputDecoration(
                      labelText: l.bodyAge,
                      suffixText: l.bodyAgeYears,
                      border: const OutlineInputBorder(),
                    ),
                  ),
                ),
            ],
          ),
          const SizedBox(height: Space.l),
          Wrap(
            spacing: Space.m,
            runSpacing: Space.m,
            children: [
              for (final e in _fields.entries)
                SizedBox(
                  width: 224,
                  child: TextField(
                    controller: e.value,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    inputFormatters: [
                      FilteringTextInputFormatter.allow(RegExp(r'[0-9.,]')),
                    ],
                    onChanged: (_) => setState(() => _preset = null),
                    onSubmitted: (_) => _apply(),
                    decoration: InputDecoration(
                      labelText: _label(l, e.key),
                      suffixText: l.cm,
                      border: const OutlineInputBorder(),
                    ),
                  ),
                ),
            ],
          ),
          if (_nearestText(l) case final size?) ...[
            const SizedBox(height: Space.s),
            Text(
              size,
              key: const Key('bodyNearestSize'),
              style: ts(fontSize: 12.5, color: p.textMuted),
            ),
          ],
          if (_error != null) ...[
            const SizedBox(height: Space.m),
            Text(_error!, style: ts(fontSize: 13, color: p.danger)),
          ],
          if (_achieved != null) ...[
            const SizedBox(height: Space.m),
            _Achieved(achieved: _achieved!),
          ],
          if (_measures.isNotEmpty) ...[
            const SizedBox(height: Space.m),
            MoreMeasurements(measures: _measures, onSet: _setMeasure),
          ],
          const SizedBox(height: Space.l),
          if (_naming)
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _name,
                    autofocus: true,
                    onSubmitted: (_) => _savePreset(),
                    decoration: InputDecoration(
                      labelText: l.presetName,
                      border: const OutlineInputBorder(),
                    ),
                  ),
                ),
                const SizedBox(width: Space.s),
                FilledButton(onPressed: _savePreset, child: Text(l.save)),
              ],
            )
          else
            Row(
              children: [
                TextButton.icon(
                  onPressed: () => setState(() => _naming = true),
                  icon: const Icon(Icons.bookmark_add_outlined, size: 18),
                  label: Text(l.saveAsPreset),
                ),
                if (_isCustom)
                  TextButton.icon(
                    onPressed: _deletePreset,
                    style: TextButton.styleFrom(foregroundColor: p.danger),
                    icon: const Icon(Icons.delete_outline_rounded, size: 18),
                    label: Text(l.deletePreset),
                  ),
                const Spacer(),
                FilledButton(onPressed: _apply, child: Text(l.apply)),
              ],
            ),
        ],
      ),
    );
  }
}

/// What the fitted body actually measures.
class _Achieved extends StatelessWidget {
  const _Achieved({required this.achieved});

  final Map<String, dynamic> achieved;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    String v(String k) =>
        '${context.number(achieved[k] as num, 1)} ${context.l.cm}';
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        color: p.surfaceMuted,
        borderRadius: BorderRadius.circular(kRadius * 0.75),
      ),
      child: Text(
        context.l.fittedBody(
          v('height_cm'),
          v('bust_cm'),
          v('waist_cm'),
          v('hips_cm'),
        ),
        style: ts(fontSize: 12.5, color: p.textMuted, tabular: true),
      ),
    );
  }
}
