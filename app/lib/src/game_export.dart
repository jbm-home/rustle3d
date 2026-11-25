// The game-engine export: the garment as the 3D view shows it, with
// levels of detail, baked normal maps, back faces or thickness, and
// skinning to the body's skeleton (docs/specs/15-game-export.md).

part of 'shell.dart';

/// The engines' defaults (as `r3d_commands::game::GamePreset`): LODs in
/// %, normal map size.
const _gamePresets = {
  'generic': ('100, 50, 20', 1024),
  'godot': ('100, 50, 20', 2048),
  'unity': ('100, 50, 20', 2048),
  'unreal': ('100, 50, 25, 12.5', 2048),
};

/// Normal map sizes offered (0: none).
const _mapSizes = [0, 512, 1024, 2048, 4096];

/// Pick the engine and the options, then where to save the `.glb`.
class _GameExportDialog extends StatefulWidget {
  const _GameExportDialog({
    required this.engine,
    required this.name,
  });

  final Engine engine;

  /// The project's name, for the file.
  final String name;

  @override
  State<_GameExportDialog> createState() => _GameExportDialogState();
}

class _GameExportDialogState extends State<_GameExportDialog> {
  String _preset = 'generic';
  final _lods = TextEditingController(text: '100, 50, 20');
  int _map = 1024;
  bool _shell = false;
  final _thickness = TextEditingController(text: '1');
  bool _skin = true;
  bool _avatar = false;
  String? _error;

  @override
  void dispose() {
    _lods.dispose();
    _thickness.dispose();
    super.dispose();
  }

  void _choose(String preset) {
    final (lods, map) = _gamePresets[preset]!;
    setState(() {
      _preset = preset;
      _lods.text = lods;
      _map = map;
    });
  }

  /// The LODs typed (in %) as ratios; null when a part is not a number.
  List<double>? get _ratios {
    final parts = _lods.text
        .split(RegExp(r'[;,\s]+'))
        .where((s) => s.isNotEmpty)
        .map(double.tryParse)
        .toList();
    if (parts.isEmpty || parts.any((p) => p == null)) return null;
    return [for (final p in parts) p! / 100];
  }

  Future<void> _export() async {
    final l = context.l;
    final ratios = _ratios;
    if (ratios == null) {
      setState(() => _error = l.errLods);
      return;
    }
    final options = jsonEncode({
      'preset': _preset,
      'lods': ratios,
      'normal_map': _map,
      'sides': _shell ? 'shell' : 'double_sided',
      if (_shell) 'thickness_mm': _parseNumber(_thickness.text) ?? 1,
      'skin': _skin,
      'avatar': _avatar,
    });
    final location = await getSaveLocation(
      suggestedName: '${widget.name}-$_preset.glb',
      acceptedTypeGroups: [
        const XTypeGroup(label: 'GLB', extensions: ['glb']),
      ],
    );
    if (location == null || !mounted) return;
    try {
      await runBusy(
        context,
        widget.engine,
        l.export,
        () =>
            widget.engine.exportGame(path: location.path, optionsJson: options),
      );
      if (mounted) {
        Navigator.pop(context, location.path.split(RegExp(r'[/\\]')).last);
      }
    } on ApiError catch (e) {
      if (mounted) setState(() => _error = context.error(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    Widget label(String text) => Padding(
      padding: const EdgeInsets.only(top: Space.m, bottom: Space.xs),
      child: Text(text, style: ts(fontSize: 13, color: p.text)),
    );
    Widget hint(String text) => Padding(
      padding: const EdgeInsets.only(top: Space.xs),
      child: Text(
        text,
        style: ts(fontSize: 12, height: 1.4, color: p.textMuted),
      ),
    );
    return AlertDialog(
      title: Text(l.gameTitle),
      content: SizedBox(
        width: 460,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                l.gameIntro,
                style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
              ),
              label(l.gameEngine),
              SegmentedButton<String>(
                key: const ValueKey('gamePreset'),
                segments: [
                  ButtonSegment(value: 'generic', label: Text(l.gameGeneric)),
                  const ButtonSegment(value: 'godot', label: Text('Godot')),
                  const ButtonSegment(value: 'unity', label: Text('Unity')),
                  const ButtonSegment(value: 'unreal', label: Text('Unreal')),
                ],
                selected: {_preset},
                onSelectionChanged: (s) => _choose(s.first),
              ),
              label(l.gameLods),
              TextField(
                key: const ValueKey('gameLods'),
                controller: _lods,
                decoration: const InputDecoration(
                  isDense: true,
                  suffixText: '%',
                ),
              ),
              hint(l.gameLodsHint),
              label(l.gameNormalMap),
              DropdownButton<int>(
                key: const ValueKey('gameMap'),
                isExpanded: true,
                value: _map,
                items: [
                  for (final s in _mapSizes)
                    DropdownMenuItem(
                      value: s,
                      child: Text(s == 0 ? l.gameNoMap : '$s × $s'),
                    ),
                ],
                onChanged: (s) => setState(() => _map = s ?? _map),
              ),
              hint(l.gameNormalMapHint),
              label(l.gameSides),
              SegmentedButton<bool>(
                key: const ValueKey('gameSides'),
                segments: [
                  ButtonSegment(value: false, label: Text(l.gameDoubleSided)),
                  ButtonSegment(value: true, label: Text(l.gameThickness)),
                ],
                selected: {_shell},
                onSelectionChanged: (s) => setState(() => _shell = s.first),
              ),
              if (_shell) ...[
                const SizedBox(height: Space.s),
                Row(
                  children: [
                    Text(
                      l.gameThicknessMm,
                      style: ts(fontSize: 13, color: p.text),
                    ),
                    const SizedBox(width: Space.s),
                    SizedBox(
                      width: 96,
                      child: _NumberField(
                        key: const Key('game-thickness'),
                        controller: _thickness,
                        unit: l.mm,
                      ),
                    ),
                  ],
                ),
              ],
              const SizedBox(height: Space.s),
              SwitchListTile(
                key: const ValueKey('gameSkin'),
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: Text(l.gameSkin),
                subtitle: Text(l.gameSkinHint),
                value: _skin,
                onChanged: (v) => setState(() => _skin = v),
              ),
              SwitchListTile(
                key: const ValueKey('gameAvatar'),
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: Text(l.gameAvatar),
                value: _avatar,
                onChanged: (v) => setState(() => _avatar = v),
              ),
              if (_error case final e?) ...[
                const SizedBox(height: Space.s),
                Text(e, style: ts(fontSize: 13, color: p.danger)),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton.icon(
          key: const ValueKey('gameExport'),
          onPressed: _export,
          icon: const Icon(Icons.sports_esports_outlined, size: 18),
          label: Text(l.gameExport),
        ),
      ],
    );
  }
}
