// The "Body" panel: choose a preset, enter measurements, fit the avatar,
// and manage the user's own presets.

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'rust/api/engine.dart';
import 'rust/api/library.dart';

/// Ranges accepted by the `avatar.set` command (cm).
const _ranges = {
  'height_cm': (120.0, 220.0),
  'bust_cm': (60.0, 170.0),
  'waist_cm': (45.0, 170.0),
  'hips_cm': (60.0, 180.0),
};

const _labels = {
  'height_cm': 'Height',
  'bust_cm': 'Bust / chest',
  'waist_cm': 'Waist',
  'hips_cm': 'Hips',
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
  Map<String, dynamic>? _achieved;
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
      _achieved = current['achieved'] as Map<String, dynamic>?;
    }
  }

  @override
  void dispose() {
    for (final c in _fields.values) {
      c.dispose();
    }
    _name.dispose();
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
  }

  Map<String, Object>? _readBody() {
    final body = <String, Object>{'masculinity': _masculinity};
    for (final e in _fields.entries) {
      final v = double.tryParse(e.value.text.replaceAll(',', '.'));
      final (min, max) = _ranges[e.key]!;
      if (v == null || v < min || v > max) {
        setState(
          () => _error =
              '${_labels[e.key]} must be between ${min.round()} and ${max.round()} cm.',
        );
        return null;
      }
      body[e.key] = v;
    }
    return body;
  }

  void _apply() {
    final body = _readBody();
    if (body == null) return;
    try {
      widget.engine.run(
        stepJson: jsonEncode({
          'type': 'avatar.set',
          'body': body,
          'preset': ?_preset,
        }),
      );
      final got = jsonDecode(
        widget.engine.run(stepJson: '{"type":"avatar.get"}'),
      );
      setState(() {
        _error = null;
        _achieved = got['achieved'] as Map<String, dynamic>;
      });
      widget.onChanged();
    } on ApiError catch (e) {
      setState(() => _error = e.text);
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
      setState(() => _error = e.text);
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

  bool get _isCustom =>
      _presets.any((p) => p['name'] == _preset && p['builtin'] == false);

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final builtin = _presets.where((x) => x['builtin'] == true);
    final mine = _presets.where((x) => x['builtin'] == false);
    return Padding(
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
                'Body',
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
            'The 3D body is fitted to these measurements. Start from a size, then adjust.',
            style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
          ),
          const SizedBox(height: Space.l),
          DropdownButtonFormField<String>(
            initialValue: _presets.any((x) => x['name'] == _preset)
                ? _preset
                : null,
            decoration: const InputDecoration(
              labelText: 'Preset',
              border: OutlineInputBorder(),
            ),
            items: [
              for (final x in builtin)
                DropdownMenuItem(
                  value: x['name'] as String,
                  child: Text(x['name'] as String),
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
          SegmentedButton<double>(
            segments: const [
              ButtonSegment(value: 0, label: Text('Feminine body')),
              ButtonSegment(value: 1, label: Text('Masculine body')),
            ],
            selected: {_masculinity < 0.5 ? 0 : 1},
            onSelectionChanged: (s) => setState(() {
              _masculinity = s.first;
              _preset = null;
            }),
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
                      labelText: _labels[e.key],
                      suffixText: 'cm',
                      border: const OutlineInputBorder(),
                    ),
                  ),
                ),
            ],
          ),
          if (_error != null) ...[
            const SizedBox(height: Space.m),
            Text(_error!, style: ts(fontSize: 13, color: p.danger)),
          ],
          if (_achieved != null) ...[
            const SizedBox(height: Space.m),
            _Achieved(achieved: _achieved!),
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
                    decoration: const InputDecoration(
                      labelText: 'Preset name',
                      border: OutlineInputBorder(),
                    ),
                  ),
                ),
                const SizedBox(width: Space.s),
                FilledButton(onPressed: _savePreset, child: const Text('Save')),
              ],
            )
          else
            Row(
              children: [
                TextButton.icon(
                  onPressed: () => setState(() => _naming = true),
                  icon: const Icon(Icons.bookmark_add_outlined, size: 18),
                  label: const Text('Save as preset'),
                ),
                if (_isCustom)
                  TextButton.icon(
                    onPressed: _deletePreset,
                    style: TextButton.styleFrom(foregroundColor: p.danger),
                    icon: const Icon(Icons.delete_outline_rounded, size: 18),
                    label: const Text('Delete preset'),
                  ),
                const Spacer(),
                FilledButton(onPressed: _apply, child: const Text('Apply')),
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
    String v(String k) => '${(achieved[k] as num).toStringAsFixed(1)} cm';
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        color: p.surfaceMuted,
        borderRadius: BorderRadius.circular(kRadius * 0.75),
      ),
      child: Text(
        'Fitted body: height ${v('height_cm')} · bust ${v('bust_cm')} · '
        'waist ${v('waist_cm')} · hips ${v('hips_cm')}',
        style: ts(fontSize: 12.5, color: p.textMuted, tabular: true),
      ),
    );
  }
}
