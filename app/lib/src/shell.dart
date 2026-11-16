// The main window: top bar, tool rail, the 2D board and the 3D view side by
// side, the inspector and the status bar (docs/ux.md §4).

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'body_panel.dart';
import 'design/tokens.dart';
import 'guide.dart';
import 'rust/api/engine.dart';
import 'start_screen.dart' show Logo;
import 'views/rust_view.dart';

class Shell extends StatefulWidget {
  const Shell({
    super.key,
    required this.engine,
    required this.libraryDir,
    required this.dark,
    required this.onToggleTheme,
    required this.onHome,
  });

  final Engine engine;

  /// The user's libraries (body presets).
  final String libraryDir;
  final bool dark;
  final VoidCallback onToggleTheme;

  /// Back to the start screen.
  final VoidCallback onHome;

  @override
  State<Shell> createState() => _ShellState();
}

class _ShellState extends State<Shell> {
  Map<String, dynamic> _summary = const {};
  List<String> _selection = const [];
  Map<String, dynamic>? _panel;
  List<dynamic> _pins = const [];
  List<dynamic> _seams = const [];
  String? _edge;
  String _tool = 'select';

  // Performance readouts, smoothed and refreshed twice a second.
  double _boardMs = 0;
  double _garmentMs = 0;
  int _frames = 0;
  double _fps = 0;
  SimStatus? _sim;
  final _clock = Stopwatch()..start();

  /// The guided flow's current step, and whether the guide is shown.
  late GuideStep _step;
  bool _guide = true;

  @override
  void initState() {
    super.initState();
    _refresh();
    _step = GuideStep.firstFor(
      panels: (_summary['counts']?['panels'] as int?) ?? 0,
      seams: _seams.length,
    );
  }

  /// Go to a step of the guided flow: its tool becomes the active one.
  void _goTo(GuideStep step) {
    setState(() {
      _step = step;
      _guide = true;
    });
    switch (step) {
      case GuideStep.sew:
        _setTool('sew');
      case GuideStep.fabric:
        // The fabric is in the inspector when nothing is selected.
        _guard(() => widget.engine.select(panels: const []));
        _setTool('select');
        _refresh();
      case _:
        _setTool('select');
    }
  }

  void _showBody() =>
      showBodyPanel(context, widget.engine, widget.libraryDir, _refresh);

  void _look(CameraSide side) =>
      _guard(() => widget.engine.lookFrom(side: side));

  /// The step's main action, from the guide banner.
  void _stepAction() {
    switch (_step) {
      case GuideStep.measurements:
        _showBody();
      case GuideStep.pattern:
        _addRectangle();
      case GuideStep.sew:
        _setTool('sew');
      case GuideStep.fabric:
        break;
      case GuideStep.drape:
        _setRunning(!(_sim?.running ?? false));
    }
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
      _pins = (_run({'type': 'pin.list'}) as List<dynamic>?) ?? const [];
      _seams = (_run({'type': 'seam.list'}) as List<dynamic>?) ?? const [];
      _edge = _guard<String?>(widget.engine.selectedEdge);
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
      setState(() {
        _fps = _frames * 1000 / elapsed;
        _sim = _guard(widget.engine.simulationStatus);
      });
      _frames = 0;
      _clock.reset();
    }
  }

  T? _guard<T>(T Function() call) {
    try {
      return call();
    } on ApiError catch (e) {
      _toast(e.text);
      return null;
    }
  }

  void _setRunning(bool running) => setState(
    () => _sim = _guard(() => widget.engine.runSimulation(running: running)),
  );

  /// Rebuild the simulation from the pattern, keeping it running if it was.
  void _restart() {
    final wasRunning = _sim?.running ?? false;
    setState(() {
      _sim = _guard(widget.engine.resetSimulation);
      if (wasRunning) {
        _sim = _guard(() => widget.engine.runSimulation(running: true));
      }
    });
  }

  /// The running drape follows the new fabric (the engine carries edits
  /// over).
  void _setFabric(String fabric) {
    _run({'type': 'garment.set_fabric', 'fabric': fabric});
    _refresh();
  }

  /// The first edge picked with the Sew tool.
  String? _sewFrom;

  void _setTool(String tool) => setState(() {
    _tool = tool;
    _sewFrom = null;
  });

  /// Sew tool: the first click picks an edge, the second sews them.
  void _sewClick() {
    final edge = _guard<String?>(widget.engine.selectedEdge);
    if (edge == null) {
      _refresh();
      return;
    }
    final from = _sewFrom;
    if (from == null || from == edge) {
      setState(() => _sewFrom = edge);
      _refresh();
      return;
    }
    final out = _run({
      'type': 'seam.add',
      'a': {'edge': from},
      'b': {'edge': edge},
    });
    setState(() => _sewFrom = null);
    _refresh();
    if (out != null) {
      _toast('Sewn. Flip it in the inspector if it looks twisted.');
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

  String _simStats() {
    final s = _sim;
    if (s == null || !s.loaded) return '';
    return 'Cloth ${s.particles} points · step ${s.stepMs.toStringAsFixed(1)} ms · '
        '${s.timeS.toStringAsFixed(1)} s · ';
  }

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.keyZ, meta: true): _undo,
        const SingleActivator(LogicalKeyboardKey.keyZ, meta: true, shift: true):
            _redo,
        const SingleActivator(LogicalKeyboardKey.keyR): _addRectangle,
        const SingleActivator(LogicalKeyboardKey.keyV): () =>
            _setTool('select'),
        const SingleActivator(LogicalKeyboardKey.keyD): () => _setTool('draw'),
        const SingleActivator(LogicalKeyboardKey.keyS): () => _setTool('sew'),
        const SingleActivator(LogicalKeyboardKey.backspace): _deleteSelected,
        const SingleActivator(LogicalKeyboardKey.digit1): () =>
            _look(CameraSide.front),
        const SingleActivator(LogicalKeyboardKey.digit2): () =>
            _look(CameraSide.left),
        const SingleActivator(LogicalKeyboardKey.digit3): () =>
            _look(CameraSide.back),
        const SingleActivator(LogicalKeyboardKey.digit4): () =>
            _look(CameraSide.threeQuarter),
        const SingleActivator(LogicalKeyboardKey.space): () =>
            _setRunning(!(_sim?.running ?? false)),
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
                onBody: _showBody,
                onHome: widget.onHome,
                step: _step,
                guide: _guide,
                onStep: _goTo,
              ),
              GuideBanner(
                step: _step,
                visible: _guide,
                running: _sim?.running ?? false,
                onAction: _stepAction,
                onNext: _step.next == null ? null : () => _goTo(_step.next!),
                onClose: () => setState(() => _guide = false),
              ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(Space.s, 0, Space.m, 0),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _ToolRail(
                        active: _tool,
                        highlight: _guide ? _step.tool : null,
                        onSelect: (tool) {
                          if (tool == 'rectangle') {
                            _addRectangle();
                          } else {
                            _setTool(tool);
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
                            onSelection: (_) =>
                                _tool == 'sew' ? _sewClick() : _refresh(),
                            onFrameTime: (ms) => _onFrame(true, ms),
                          ),
                        ),
                      ),
                      const SizedBox(width: Space.m),
                      Expanded(
                        child: _ViewCard(
                          title: '3D',
                          icon: Icons.view_in_ar_outlined,
                          trailing: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              _CameraBar(onLook: _look),
                              const SizedBox(width: Space.s),
                              _SimControls(
                                status: _sim,
                                onRun: _setRunning,
                                onRestart: _restart,
                              ),
                            ],
                          ),
                          child: RustView(
                            engine: widget.engine,
                            kind: ViewKind.garment,
                            onSelection: (_) => _refresh(),
                            onFrameTime: (ms) => _onFrame(false, ms),
                            onPinned: (_) {
                              _refresh();
                              _toast('Pinned. Remove it in the inspector.');
                            },
                            drawing: _tool == 'draw',
                            onDrawn: (_) {
                              _refresh();
                              _toast('Line added to the pattern.');
                            },
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
                          edge: _edge,
                          seams: _seams,
                          onFabric: _setFabric,
                          pins: _pins,
                          onUnpin: (pin) {
                            _guard(() => widget.engine.unpin(pin: pin));
                            _refresh();
                          },
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
                hint: switch (_tool) {
                  'sew' when _sewFrom == null =>
                    'Sew: click the first edge on the pattern · V to stop',
                  'sew' => 'Sew: now click the edge to sew it to · V to stop',
                  'draw' => 'Draw: drag on the 3D garment; the line goes on the pattern · V to stop',
                  _ when _selection.isEmpty => 'In 3D: drag to pull the cloth · ⇧-drag a hem to lengthen, a seam to take in · ⌥-click to pin',
                  _ => 'Adjust the selection in the inspector · ⌫ deletes · ⌘Z undoes',
                },
                stats:
                    '${_simStats()}Rust render · pattern ${_boardMs.toStringAsFixed(1)} ms · '
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
    required this.onBody,
    required this.onHome,
    required this.step,
    required this.guide,
    required this.onStep,
  });

  final String projectName;
  final bool dark;
  final VoidCallback onToggleTheme;
  final VoidCallback onUndo;
  final VoidCallback onRedo;
  final VoidCallback onBody;
  final VoidCallback onHome;
  final GuideStep step;

  /// The guide is open: the current step is shown as active.
  final bool guide;
  final ValueChanged<GuideStep> onStep;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    const steps = GuideStep.values;
    return SizedBox(
      height: 56,
      child: Padding(
        // Leave room for the macOS window buttons (transparent title bar).
        padding: const EdgeInsets.only(left: 84, right: Space.m),
        child: Row(
          children: [
            Tooltip(
              message: 'All projects',
              child: InkWell(
                onTap: onHome,
                borderRadius: BorderRadius.circular(8),
                child: Padding(
                  padding: const EdgeInsets.all(4),
                  child: Row(
                    children: [
                      Icon(
                        Icons.chevron_left_rounded,
                        size: 18,
                        color: p.textMuted,
                      ),
                      const Logo(),
                    ],
                  ),
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
              _StepChip(
                index: i + 1,
                label: steps[i].label,
                active: guide && steps[i] == step,
                done: steps[i].index < step.index,
                onTap: () => onStep(steps[i]),
              ),
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
            TextButton.icon(
              onPressed: onBody,
              icon: const Icon(Icons.accessibility_new_rounded, size: 18),
              label: const Text('Body'),
            ),
            const SizedBox(width: Space.s),
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
    required this.done,
    required this.onTap,
  });

  final int index;
  final String label;
  final bool active;

  /// Before the current step: shown with a check mark.
  final bool done;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(999),
      child: AnimatedContainer(
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
                color: active
                    ? p.accent
                    : (done ? p.accent.withValues(alpha: 0.18) : p.border),
              ),
              child: done && !active
                  ? Icon(Icons.check_rounded, size: 12, color: p.accent)
                  : Text(
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
  const _ToolRail({
    required this.active,
    required this.highlight,
    required this.onSelect,
  });

  final String active;

  /// The tool the guided flow's current step uses, ringed.
  final String? highlight;
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
    (
      'draw',
      Icons.brush_rounded,
      'Draw on garment',
      'D',
      'Drag on the 3D garment: the line goes on the pattern',
    ),
    ('notch', Icons.vertical_align_center_rounded, 'Notch', 'N', 'Coming soon'),
    ('sew', Icons.link_rounded, 'Sew', 'S', 'Click two edges on the pattern'),
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
                  highlight: id == highlight,
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
    required this.highlight,
    required this.onTap,
  });

  final IconData icon;
  final bool active;
  final bool highlight;
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
            border: Border.all(
              color: widget.highlight
                  ? p.accent.withValues(alpha: 0.7)
                  : Colors.transparent,
              width: 1.5,
            ),
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
    this.trailing,
  });

  final String title;
  final IconData icon;
  final Widget child;
  final Widget? trailing;

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
          if (trailing != null)
            Positioned(right: Space.m, top: Space.m, child: trailing!),
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
    required this.edge,
    required this.seams,
    required this.onFabric,
    required this.pins,
    required this.onUnpin,
    required this.run,
    required this.onDelete,
  });

  final List<String> selection;
  final Map<String, dynamic>? panel;
  final Map<String, dynamic> summary;
  final String? edge;
  final List<dynamic> seams;
  final ValueChanged<String> onFabric;
  final List<dynamic> pins;
  final ValueChanged<String> onUnpin;
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
            0 => _EmptyInspector(
              summary: summary,
              onFabric: onFabric,
              onArrange: () => run({'type': 'garment.arrange'}),
              pins: pins,
              onUnpin: onUnpin,
            ),
            1 when panel != null => _PanelInspector(
              panel: panel!,
              edge: edge,
              seams: seams,
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
  const _EmptyInspector({
    required this.summary,
    required this.onFabric,
    required this.onArrange,
    required this.pins,
    required this.onUnpin,
  });

  final Map<String, dynamic> summary;
  final ValueChanged<String> onFabric;
  final VoidCallback onArrange;
  final List<dynamic> pins;
  final ValueChanged<String> onUnpin;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final count = summary['counts']?['panels'] ?? 0;
    return ListView(
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
        const SizedBox(height: Space.xl),
        Divider(color: p.border, height: 1),
        const SizedBox(height: Space.l),
        _FabricPicker(
          current:
              ((summary['garments'] as List<dynamic>?)?.firstOrNull
                      as Map<String, dynamic>?)?['fabric']
                  as String? ??
              'cotton',
          onPick: onFabric,
        ),
        const SizedBox(height: Space.l),
        OutlinedButton.icon(
          onPressed: onArrange,
          icon: const Icon(Icons.threesixty_rounded, size: 18),
          label: const Text('Arrange around the body'),
        ),
        const SizedBox(height: Space.xs),
        Text(
          'Places every piece around the body in seam order, at the waist.',
          style: ts(fontSize: 12, height: 1.4, color: p.textMuted),
        ),
        const SizedBox(height: Space.l),
        Divider(color: p.border, height: 1),
        const SizedBox(height: Space.l),
        _PinList(pins: pins, onUnpin: onUnpin),
      ],
    );
  }
}

/// The pins holding the cloth, each removable.
class _PinList extends StatelessWidget {
  const _PinList({required this.pins, required this.onUnpin});

  final List<dynamic> pins;
  final ValueChanged<String> onUnpin;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Pins',
          style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Text(
          pins.isEmpty
              ? '⌥-click the cloth in 3D to hold it in place.'
              : '⌥-click the cloth in 3D to add one.',
          style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
        ),
        for (final pin in pins)
          Padding(
            padding: const EdgeInsets.only(top: Space.s),
            child: Row(
              children: [
                Icon(Icons.push_pin_rounded, size: 16, color: p.accent),
                const SizedBox(width: Space.s),
                Expanded(
                  child: Text(
                    '${pin['panel_name'] ?? pin['panel']}',
                    style: ts(fontSize: 13, color: p.text),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                _IconAction(
                  icon: Icons.close_rounded,
                  tip: 'Remove this pin',
                  onTap: () => onUnpin(pin['id'] as String),
                ),
              ],
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
    required this.edge,
    required this.seams,
    required this.run,
    required this.onDelete,
  });

  final Map<String, dynamic> panel;
  final String? edge;
  final List<dynamic> seams;
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
        if ((panel['edges'] as List).firstWhere(
              (e) => e['id'] == edge,
              orElse: () => null,
            )
            case final Map<String, dynamic> e) ...[
          const SizedBox(height: Space.m),
          _EdgeCard(edge: e, seams: seams, run: run),
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

/// Look at the body from the front, the side, the back or three quarters.
class _CameraBar extends StatelessWidget {
  const _CameraBar({required this.onLook});

  final ValueChanged<CameraSide> onLook;

  static const _sides = [
    (CameraSide.front, 'Front', '1'),
    (CameraSide.left, 'Side', '2'),
    (CameraSide.back, 'Back', '3'),
    (CameraSide.threeQuarter, '¾', '4'),
  ];

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        color: p.surface.withValues(alpha: 0.85),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: p.border),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (final (side, label, key) in _sides)
            Tooltip(
              message:
                  'View from the ${label == '¾' ? 'three-quarter angle' : label.toLowerCase()}  $key',
              child: TextButton(
                onPressed: () => onLook(side),
                style: TextButton.styleFrom(
                  visualDensity: VisualDensity.compact,
                  minimumSize: const Size(36, 32),
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  foregroundColor: p.textMuted,
                  shape: const StadiumBorder(),
                ),
                child: Text(
                  label,
                  style: ts(fontSize: 12.5, fontWeight: FontWeight.w600),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Simulate / pause and restart, over the 3D view.
class _SimControls extends StatelessWidget {
  const _SimControls({
    required this.status,
    required this.onRun,
    required this.onRestart,
  });

  final SimStatus? status;
  final ValueChanged<bool> onRun;
  final VoidCallback onRestart;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final running = status?.running ?? false;
    return Container(
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        color: p.surface.withValues(alpha: 0.85),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: p.border),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Tooltip(
            message: running ? 'Pause  Space' : 'Drape the garment  Space',
            child: FilledButton.icon(
              onPressed: () => onRun(!running),
              style: FilledButton.styleFrom(
                visualDensity: VisualDensity.compact,
                padding: const EdgeInsets.symmetric(horizontal: 14),
              ),
              icon: Icon(
                running ? Icons.pause_rounded : Icons.play_arrow_rounded,
                size: 18,
              ),
              label: Text(running ? 'Pause' : 'Simulate'),
            ),
          ),
          const SizedBox(width: 2),
          _IconAction(
            icon: Icons.restart_alt_rounded,
            tip: 'Start again from the pattern',
            onTap: onRestart,
          ),
        ],
      ),
    );
  }
}

/// The garment's fabric, as swatches in the colour it renders with.
class _FabricPicker extends StatelessWidget {
  const _FabricPicker({required this.current, required this.onPick});

  final String current;
  final ValueChanged<String> onPick;

  static const _blurbs = {
    'cotton': 'Crisp and breathable',
    'denim': 'Heavy, holds its shape',
    'jersey': 'Stretchy knit',
    'silk': 'Light and fluid',
    'wool': 'Soft, with body',
  };

  static String _label(String id) => id[0].toUpperCase() + id.substring(1);

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final swatches = Engine.fabrics();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Fabric',
          style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Text(
          '${_label(current)} · ${_blurbs[current] ?? ''}',
          style: ts(fontSize: 12.5, color: p.textMuted),
        ),
        const SizedBox(height: Space.m),
        Wrap(
          spacing: Space.s,
          runSpacing: Space.s,
          children: [
            for (final f in swatches)
              Tooltip(
                message: _label(f.id),
                child: InkWell(
                  onTap: () => onPick(f.id),
                  customBorder: const CircleBorder(),
                  child: AnimatedContainer(
                    duration: Motion.fast,
                    width: 34,
                    height: 34,
                    padding: const EdgeInsets.all(3),
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(
                        color: f.id == current ? p.accent : p.border,
                        width: f.id == current ? 2 : 1,
                      ),
                    ),
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: Color(f.argb),
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ],
    );
  }
}

/// The edge picked in either view: its length and its seam.
class _EdgeCard extends StatelessWidget {
  const _EdgeCard({required this.edge, required this.seams, required this.run});

  final Map<String, dynamic> edge;
  final List<dynamic> seams;
  final ValueChanged<Map<String, Object?>> run;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final id = edge['id'] as String;
    final length = (edge['length_mm'] as num) / 10;
    final seam = seams.firstWhere(
      (s) => s['a']['edge'] == id || s['b']['edge'] == id,
      orElse: () => null,
    );
    final other = seam == null
        ? null
        : (seam['a']['edge'] == id ? seam['b'] : seam['a']);
    return Container(
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        color: p.accentSoft,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.timeline_rounded, size: 16, color: p.accent),
              const SizedBox(width: Space.s),
              Text(
                'Edge · ${length.toStringAsFixed(1)} cm',
                style: ts(
                  fontSize: 13.5,
                  fontWeight: FontWeight.w600,
                  color: p.text,
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.xs),
          Text(
            other == null
                ? 'Open edge (a hem or an opening). ⇧-drag it in 3D to lengthen.'
                : 'Sewn to ${other['panel_name'] ?? other['panel']} · '
                      '${(seam['mismatch_mm'] as num).toStringAsFixed(1)} mm difference. '
                      '⇧-drag it in 3D to take in.',
            style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
          ),
          if (seam != null)
            Row(
              children: [
                TextButton(
                  onPressed: () =>
                      run({'type': 'seam.flip', 'seam': seam['id']}),
                  child: const Text('Flip seam'),
                ),
                TextButton(
                  onPressed: () =>
                      run({'type': 'seam.remove', 'seam': seam['id']}),
                  child: const Text('Unpick'),
                ),
              ],
            ),
        ],
      ),
    );
  }
}
