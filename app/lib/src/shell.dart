// The main window: top bar, tool rail, the 2D board and the 3D view side by
// side, the inspector and the status bar (docs/ux.md §4).

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'rust/api/engine.dart';
import 'views/rust_view.dart';

class Shell extends StatefulWidget {
  const Shell({
    super.key,
    required this.engine,
    required this.dark,
    required this.onToggleTheme,
  });

  final Engine engine;
  final bool dark;
  final VoidCallback onToggleTheme;

  @override
  State<Shell> createState() => _ShellState();
}

class _ShellState extends State<Shell> {
  Map<String, dynamic> _summary = const {};
  List<String> _selection = const [];
  Map<String, dynamic>? _panel;
  String _tool = 'select';

  // Performance readouts, smoothed and refreshed twice a second.
  double _boardMs = 0;
  double _garmentMs = 0;
  int _frames = 0;
  double _fps = 0;
  final _clock = Stopwatch()..start();

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  dynamic _run(Map<String, Object?> step) {
    try {
      return jsonDecode(widget.engine.run(stepJson: jsonEncode(step)));
    } on ApiError catch (e) {
      _toast(e.text);
      return null;
    }
  }

  void _toast(String text) {
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(content: Text(text), behavior: SnackBarBehavior.floating),
      );
  }

  void _refresh() {
    setState(() {
      _summary =
          (_run({'type': 'project.summary'}) as Map<String, dynamic>?) ??
          const {};
      _selection = widget.engine.selection();
      _panel = _selection.length == 1
          ? _run({'type': 'panel.get', 'panel': _selection.first})
                as Map<String, dynamic>?
          : null;
    });
  }

  void _onFrame(bool board, double ms) {
    if (board) {
      _boardMs = _boardMs * 0.9 + ms * 0.1;
      return;
    }
    _garmentMs = _garmentMs * 0.9 + ms * 0.1;
    _frames++;
    final elapsed = _clock.elapsedMilliseconds;
    if (elapsed >= 500) {
      setState(() => _fps = _frames * 1000 / elapsed);
      _frames = 0;
      _clock.reset();
    }
  }

  void _addRectangle() {
    final panels = (_summary['panels'] as List<dynamic>? ?? const []);
    var right = 0.0;
    for (final p in panels) {
      final max = (p['bbox']?['max'] as List<dynamic>?)?.first as num?;
      if (max != null && max > right) right = max.toDouble();
    }
    _run({
      'type': 'panel.create_rect',
      'origin': [right + 60, 0],
      'width': 300,
      'height': 400,
    });
    _refresh();
  }

  void _undo() {
    _run({'type': 'history.undo'});
    _refresh();
  }

  void _redo() {
    _run({'type': 'history.redo'});
    _refresh();
  }

  Future<void> _deleteSelected() async {
    final panel = _panel;
    if (panel == null) return;
    final ok = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Delete “${panel['name']}”?'),
        content: const Text('You can undo this with ⌘Z.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (ok == true) {
      _run({'type': 'panel.delete', 'panel': panel['id'], 'confirm': true});
      _refresh();
    }
  }

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.keyZ, meta: true): _undo,
        const SingleActivator(LogicalKeyboardKey.keyZ, meta: true, shift: true):
            _redo,
        const SingleActivator(LogicalKeyboardKey.keyR): _addRectangle,
        const SingleActivator(LogicalKeyboardKey.backspace): _deleteSelected,
      },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          body: Column(
            children: [
              _TopBar(
                projectName: (_summary['name'] as String?) ?? '',
                dark: widget.dark,
                onToggleTheme: widget.onToggleTheme,
                onUndo: _undo,
                onRedo: _redo,
              ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(Space.s, 0, Space.m, 0),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _ToolRail(
                        active: _tool,
                        onSelect: (tool) {
                          if (tool == 'rectangle') {
                            _addRectangle();
                          } else {
                            setState(() => _tool = tool);
                          }
                        },
                      ),
                      Expanded(
                        child: _ViewCard(
                          title: 'Pattern',
                          icon: Icons.architecture_outlined,
                          child: RustView(
                            engine: widget.engine,
                            kind: ViewKind.board,
                            onSelection: (_) => _refresh(),
                            onFrameTime: (ms) => _onFrame(true, ms),
                          ),
                        ),
                      ),
                      const SizedBox(width: Space.m),
                      Expanded(
                        child: _ViewCard(
                          title: '3D',
                          icon: Icons.view_in_ar_outlined,
                          child: RustView(
                            engine: widget.engine,
                            kind: ViewKind.garment,
                            onSelection: (_) => _refresh(),
                            onFrameTime: (ms) => _onFrame(false, ms),
                          ),
                        ),
                      ),
                      const SizedBox(width: Space.m),
                      SizedBox(
                        width: 264,
                        child: _Inspector(
                          selection: _selection,
                          panel: _panel,
                          summary: _summary,
                          run: (step) {
                            _run(step);
                            _refresh();
                          },
                          onDelete: _deleteSelected,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              _StatusBar(
                hint: _selection.isEmpty
                    ? 'Click a panel to select it · drag to pan · scroll to zoom · R adds a rectangle'
                    : 'Adjust the selection in the inspector · ⌫ deletes · ⌘Z undoes',
                stats:
                    'Rust render · pattern ${_boardMs.toStringAsFixed(1)} ms · '
                    '3D ${_garmentMs.toStringAsFixed(1)} ms · ${_fps.toStringAsFixed(0)} fps',
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _TopBar extends StatelessWidget {
  const _TopBar({
    required this.projectName,
    required this.dark,
    required this.onToggleTheme,
    required this.onUndo,
    required this.onRedo,
  });

  final String projectName;
  final bool dark;
  final VoidCallback onToggleTheme;
  final VoidCallback onUndo;
  final VoidCallback onRedo;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    const steps = ['Pattern', 'Sew', 'Fabric', 'Drape'];
    return SizedBox(
      height: 56,
      child: Padding(
        // Leave room for the macOS window buttons (transparent title bar).
        padding: const EdgeInsets.only(left: 84, right: Space.m),
        child: Row(
          children: [
            Container(
              width: 26,
              height: 26,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(7),
                gradient: LinearGradient(
                  colors: [
                    p.accent,
                    Color.lerp(p.accent, const Color(0xFFE879A6), 0.6)!,
                  ],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
              ),
              alignment: Alignment.center,
              child: Text(
                'R',
                style: ts(
                  color: Colors.white,
                  fontWeight: FontWeight.w700,
                  fontSize: 14,
                ),
              ),
            ),
            const SizedBox(width: Space.m),
            Text(
              projectName,
              style: ts(
                fontSize: 17,
                fontWeight: FontWeight.w600,
                color: p.text,
              ),
            ),
            const SizedBox(width: Space.xl),
            for (var i = 0; i < steps.length; i++) ...[
              _StepChip(index: i + 1, label: steps[i], active: i == 0),
              if (i < steps.length - 1)
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: Space.xs),
                  child: Icon(
                    Icons.chevron_right,
                    size: 16,
                    color: p.textMuted,
                  ),
                ),
            ],
            const Spacer(),
            _IconAction(
              icon: Icons.undo_rounded,
              tip: 'Undo  ⌘Z',
              onTap: onUndo,
            ),
            _IconAction(
              icon: Icons.redo_rounded,
              tip: 'Redo  ⇧⌘Z',
              onTap: onRedo,
            ),
            const SizedBox(width: Space.s),
            _IconAction(
              icon: dark ? Icons.light_mode_outlined : Icons.dark_mode_outlined,
              tip: dark ? 'Light theme' : 'Dark theme',
              onTap: onToggleTheme,
            ),
          ],
        ),
      ),
    );
  }
}

class _StepChip extends StatelessWidget {
  const _StepChip({
    required this.index,
    required this.label,
    required this.active,
  });

  final int index;
  final String label;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return AnimatedContainer(
      duration: Motion.normal,
      curve: Motion.curve,
      padding: const EdgeInsets.symmetric(horizontal: Space.m, vertical: 6),
      decoration: BoxDecoration(
        color: active ? p.accentSoft : Colors.transparent,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 18,
            height: 18,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: active ? p.accent : p.border,
            ),
            child: Text(
              '$index',
              style: ts(
                fontSize: 11,
                fontWeight: FontWeight.w700,
                color: active ? Colors.white : p.textMuted,
              ),
            ),
          ),
          const SizedBox(width: 6),
          Text(
            label,
            style: ts(
              fontSize: 13,
              fontWeight: active ? FontWeight.w600 : FontWeight.w500,
              color: active ? p.accent : p.textMuted,
            ),
          ),
        ],
      ),
    );
  }
}

class _IconAction extends StatelessWidget {
  const _IconAction({
    required this.icon,
    required this.tip,
    required this.onTap,
  });

  final IconData icon;
  final String tip;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Tooltip(
      message: tip,
      child: IconButton(
        onPressed: onTap,
        icon: Icon(icon, size: 19, color: p.textMuted),
        style: IconButton.styleFrom(
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(Tokens.radius(context, 0.5)),
          ),
        ),
      ),
    );
  }
}

class _ToolRail extends StatelessWidget {
  const _ToolRail({required this.active, required this.onSelect});

  final String active;
  final ValueChanged<String> onSelect;

  static const _tools = [
    (
      'select',
      Icons.near_me_outlined,
      'Select',
      'V',
      'Click a panel or point; drag to move',
    ),
    (
      'rectangle',
      Icons.crop_square_rounded,
      'Rectangle',
      'R',
      'Add a rectangular panel',
    ),
    ('polygon', Icons.pentagon_outlined, 'Polygon', 'P', 'Coming soon'),
    ('curve', Icons.gesture_rounded, 'Curve', 'C', 'Coming soon'),
    ('notch', Icons.vertical_align_center_rounded, 'Notch', 'N', 'Coming soon'),
    ('sew', Icons.link_rounded, 'Sew', 'S', 'Coming soon'),
  ];

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      width: 52,
      margin: const EdgeInsets.only(right: Space.s),
      padding: const EdgeInsets.symmetric(vertical: Space.s),
      decoration: BoxDecoration(
        color: p.surface,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        border: Border.all(color: p.border),
      ),
      child: Column(
        children: [
          for (final (id, icon, name, key, hint) in _tools)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 2),
              child: Tooltip(
                richMessage: TextSpan(
                  children: [
                    TextSpan(
                      text: '$name  ',
                      style: ts(fontWeight: FontWeight.w600),
                    ),
                    TextSpan(
                      text: key,
                      style: ts(color: p.surface.withValues(alpha: 0.6)),
                    ),
                    TextSpan(text: '\n$hint'),
                  ],
                ),
                preferBelow: false,
                child: _RailButton(
                  icon: icon,
                  active: id == active,
                  onTap: () => onSelect(id),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _RailButton extends StatefulWidget {
  const _RailButton({
    required this.icon,
    required this.active,
    required this.onTap,
  });

  final IconData icon;
  final bool active;
  final VoidCallback onTap;

  @override
  State<_RailButton> createState() => _RailButtonState();
}

class _RailButtonState extends State<_RailButton> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final bg = widget.active
        ? p.accentSoft
        : (_hover ? p.surfaceMuted : Colors.transparent);
    return MouseRegion(
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: Motion.fast,
          curve: Motion.curve,
          width: 38,
          height: 38,
          decoration: BoxDecoration(
            color: bg,
            borderRadius: BorderRadius.circular(Tokens.radius(context, 0.75)),
          ),
          child: Icon(
            widget.icon,
            size: 20,
            color: widget.active ? p.accent : p.textMuted,
          ),
        ),
      ),
    );
  }
}

class _ViewCard extends StatelessWidget {
  const _ViewCard({
    required this.title,
    required this.icon,
    required this.child,
  });

  final String title;
  final IconData icon;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      decoration: BoxDecoration(
        color: p.surface,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        border: Border.all(color: p.border),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.04),
            blurRadius: 16,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      clipBehavior: Clip.antiAlias,
      child: Stack(
        children: [
          Positioned.fill(child: child),
          Positioned(
            left: Space.m,
            top: Space.m,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
              decoration: BoxDecoration(
                color: p.surface.withValues(alpha: 0.85),
                borderRadius: BorderRadius.circular(999),
                border: Border.all(color: p.border),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(icon, size: 14, color: p.textMuted),
                  const SizedBox(width: 6),
                  Text(
                    title,
                    style: ts(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: p.textMuted,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Inspector extends StatelessWidget {
  const _Inspector({
    required this.selection,
    required this.panel,
    required this.summary,
    required this.run,
    required this.onDelete,
  });

  final List<String> selection;
  final Map<String, dynamic>? panel;
  final Map<String, dynamic> summary;
  final ValueChanged<Map<String, Object?>> run;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      decoration: BoxDecoration(
        color: p.surface,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        border: Border.all(color: p.border),
      ),
      padding: const EdgeInsets.all(Space.l),
      child: AnimatedSwitcher(
        duration: Motion.normal,
        child: KeyedSubtree(
          key: ValueKey(selection.join(',')),
          child: switch (selection.length) {
            0 => _EmptyInspector(summary: summary),
            1 when panel != null => _PanelInspector(
              panel: panel!,
              run: run,
              onDelete: onDelete,
            ),
            _ => _MultiInspector(count: selection.length),
          },
        ),
      ),
    );
  }
}

class _EmptyInspector extends StatelessWidget {
  const _EmptyInspector({required this.summary});

  final Map<String, dynamic> summary;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final count = summary['counts']?['panels'] ?? 0;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Inspector',
          style: ts(fontSize: 19, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xl),
        Center(
          child: Container(
            width: 64,
            height: 64,
            decoration: BoxDecoration(
              color: p.accentSoft,
              shape: BoxShape.circle,
            ),
            child: Icon(Icons.touch_app_outlined, color: p.accent, size: 28),
          ),
        ),
        const SizedBox(height: Space.l),
        Center(
          child: Text(
            'Select a panel',
            style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
          ),
        ),
        const SizedBox(height: Space.xs),
        Center(
          child: Text(
            'Click a piece on the pattern or the garment\nto adjust it here. $count pieces in this project.',
            textAlign: TextAlign.center,
            style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
          ),
        ),
      ],
    );
  }
}

class _MultiInspector extends StatelessWidget {
  const _MultiInspector({required this.count});

  final int count;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Whole garment',
          style: ts(fontSize: 15, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.s),
        Text(
          '$count pieces selected. Click one piece on the pattern to edit it.',
          style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
        ),
      ],
    );
  }
}

class _PanelInspector extends StatelessWidget {
  const _PanelInspector({
    required this.panel,
    required this.run,
    required this.onDelete,
  });

  final Map<String, dynamic> panel;
  final ValueChanged<Map<String, Object?>> run;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final id = panel['id'] as String;
    final linked = panel['mirror_of'] != null;
    final area = (panel['area_mm2'] as num) / 1e6;
    final perimeter = (panel['perimeter_mm'] as num) / 10;
    final allowance = (panel['seam_allowance_mm'] as num).toDouble();
    final grain = (panel['grain_angle_deg'] as num).toDouble();
    return ListView(
      children: [
        Text('Piece', style: ts(fontSize: 12, color: p.textMuted)),
        const SizedBox(height: Space.xs),
        Text(
          panel['name'] as String,
          style: ts(fontSize: 24, fontWeight: FontWeight.w600, color: p.text),
        ),
        if (linked) ...[
          const SizedBox(height: Space.s),
          _Badge(
            icon: Icons.flip_rounded,
            text: 'Mirror copy — follows its source',
            action: 'Unlink',
            onAction: () => run({'type': 'panel.unlink', 'panel': id}),
          ),
        ],
        const SizedBox(height: Space.l),
        Row(
          children: [
            Expanded(
              child: _Stat(
                label: 'Area',
                value: '${area.toStringAsFixed(3)} m²',
              ),
            ),
            const SizedBox(width: Space.s),
            Expanded(
              child: _Stat(
                label: 'Perimeter',
                value: '${perimeter.toStringAsFixed(1)} cm',
              ),
            ),
          ],
        ),
        const SizedBox(height: Space.xl),
        _SliderRow(
          label: 'Seam allowance',
          value: allowance,
          min: 0,
          max: 30,
          unit: 'mm',
          enabled: !linked,
          onChanged: (v) => run({
            'type': 'panel.set_seam_allowance',
            'panel': id,
            'mm': v.roundToDouble(),
          }),
        ),
        const SizedBox(height: Space.l),
        _SliderRow(
          label: 'Grainline',
          value: grain.clamp(0, 180),
          min: 0,
          max: 180,
          unit: '°',
          enabled: !linked,
          onChanged: (v) => run({
            'type': 'panel.set_grain',
            'panel': id,
            'angle_deg': v.roundToDouble(),
          }),
        ),
        const SizedBox(height: Space.xl),
        Row(
          children: [
            Expanded(
              child: OutlinedButton.icon(
                onPressed: linked
                    ? null
                    : () => run({'type': 'panel.mirror', 'panel': id}),
                icon: const Icon(Icons.flip_rounded, size: 16),
                label: const Text('Mirror'),
              ),
            ),
            const SizedBox(width: Space.s),
            Expanded(
              child: OutlinedButton.icon(
                onPressed: onDelete,
                style: OutlinedButton.styleFrom(foregroundColor: p.danger),
                icon: const Icon(Icons.delete_outline_rounded, size: 16),
                label: const Text('Delete'),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class _Stat extends StatelessWidget {
  const _Stat({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        color: p.surfaceMuted,
        borderRadius: BorderRadius.circular(Tokens.radius(context, 0.75)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: ts(fontSize: 11.5, color: p.textMuted)),
          const SizedBox(height: 2),
          Text(
            value,
            style: ts(
              fontSize: 15,
              fontWeight: FontWeight.w600,
              color: p.text,
              tabular: true,
            ),
          ),
        ],
      ),
    );
  }
}

class _SliderRow extends StatelessWidget {
  const _SliderRow({
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.unit,
    required this.enabled,
    required this.onChanged,
  });

  final String label;
  final double value;
  final double min;
  final double max;
  final String unit;
  final bool enabled;
  final ValueChanged<double> onChanged;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(
              label,
              style: ts(
                fontSize: 13,
                fontWeight: FontWeight.w500,
                color: p.text,
              ),
            ),
            const Spacer(),
            Text(
              '${value.toStringAsFixed(0)} $unit',
              style: ts(fontSize: 13, color: p.textMuted, tabular: true),
            ),
          ],
        ),
        Slider(
          value: value,
          min: min,
          max: max,
          onChanged: enabled ? onChanged : null,
        ),
      ],
    );
  }
}

class _Badge extends StatelessWidget {
  const _Badge({
    required this.icon,
    required this.text,
    required this.action,
    required this.onAction,
  });

  final IconData icon;
  final String text;
  final String action;
  final VoidCallback onAction;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      padding: const EdgeInsets.fromLTRB(Space.m, Space.xs, Space.xs, Space.xs),
      decoration: BoxDecoration(
        color: p.accentSoft,
        borderRadius: BorderRadius.circular(Tokens.radius(context, 0.5)),
      ),
      child: Row(
        children: [
          Icon(icon, size: 14, color: p.accent),
          const SizedBox(width: 6),
          Expanded(
            child: Text(text, style: ts(fontSize: 12, color: p.accent)),
          ),
          TextButton(onPressed: onAction, child: Text(action)),
        ],
      ),
    );
  }
}

class _StatusBar extends StatelessWidget {
  const _StatusBar({required this.hint, required this.stats});

  final String hint;
  final String stats;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final style = ts(fontSize: 12, color: p.textMuted);
    return SizedBox(
      height: 34,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: Space.l),
        child: Row(
          children: [
            Icon(Icons.lightbulb_outline_rounded, size: 14, color: p.textMuted),
            const SizedBox(width: 6),
            Expanded(
              child: Text(hint, style: style, overflow: TextOverflow.ellipsis),
            ),
            Text(
              stats,
              style: style.copyWith(
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
