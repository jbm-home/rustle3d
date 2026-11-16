// The main window: top bar, tool rail, the 2D board and the 3D view side by
// side, the inspector and the status bar (docs/ux.md §4).

import 'dart:convert';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'body_panel.dart';
import 'design/tokens.dart';
import 'guide.dart';
import 'i18n.dart';
import 'learn.dart';
import 'tour.dart';
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
    required this.language,
    required this.onLanguage,
    required this.onHome,
  });

  final Engine engine;

  /// The user's libraries (body presets).
  final String libraryDir;
  final bool dark;
  final VoidCallback onToggleTheme;

  /// The UI language's code, and how to change it.
  final String language;
  final ValueChanged<String> onLanguage;

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

  /// AI agents: whether they may connect, and the changes they made (the
  /// views refresh when it moves).
  AgentStatus _agent = AgentStatus.off;
  BigInt _agentChanges = BigInt.zero;

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
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _maybeTour();
    });
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

  final _scaffold = GlobalKey<ScaffoldState>();

  // What the first-launch tour points at.
  final _tourTop = GlobalKey();
  final _tourGuide = GlobalKey();
  final _tourPattern = GlobalKey();
  final _tourGarment = GlobalKey();

  // What Learn cards point at, besides the tour's areas.
  final _spotInspector = GlobalKey();
  final _spotSimulate = GlobalKey();
  final _spotCameras = GlobalKey();
  final _spotBody = GlobalKey();
  final _spotTools = {'sew': GlobalKey(), 'draw': GlobalKey()};

  void _maybeTour() {
    final memory = TourMemory(widget.libraryDir);
    if (memory.seen) return;
    memory.markSeen();
    final l = context.l;
    showTour(context, [
      TourStop(_tourPattern, l.tourPatternTitle, l.tourPatternText),
      TourStop(_tourGarment, l.tourGarmentTitle, l.tourGarmentText),
      TourStop(_tourGuide, l.tourGuideTitle, l.tourGuideText),
      TourStop(_tourTop, l.tourHelpTitle, l.tourHelpText),
    ]);
  }

  /// A Learn card's coach marks. Only the selection may change: the
  /// garment's own settings (fabric, pins) show with nothing selected.
  void _showLearn(List<LearnStop> stops) {
    if (stops.any((s) => s.$1 == Spot.garmentInspector)) {
      _guard(() => widget.engine.select(panels: const []));
      _refresh();
    }
    showTour(context, [
      for (final (spot, title, text) in stops)
        TourStop(
          switch (spot) {
            Spot.pattern => _tourPattern,
            Spot.garment => _tourGarment,
            Spot.inspector || Spot.garmentInspector => _spotInspector,
            Spot.simulate => _spotSimulate,
            Spot.cameras => _spotCameras,
            Spot.body => _spotBody,
            Spot.sewTool => _spotTools['sew']!,
            Spot.drawTool => _spotTools['draw']!,
          },
          title,
          text,
        ),
    ], done: context.l.gotIt);
  }

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
      _toast(context.error(e));
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
      _points = _guard(widget.engine.selectedPoints) ?? const [];
    });
  }

  void _onFrame(bool board, double ms) {
    if (board) {
      _boardMs = _boardMs * 0.9 + ms * 0.1;
      return;
    }
    _garmentMs = _garmentMs * 0.9 + ms * 0.1;
    _frames++;
    // An agent's change: show it at once.
    final changes = _guard(widget.engine.agentChanges);
    if (changes != null && changes != _agentChanges) {
      _agentChanges = changes;
      _refresh();
    }
    final elapsed = _clock.elapsedMilliseconds;
    if (elapsed >= 500) {
      setState(() {
        _fps = _frames * 1000 / elapsed;
        _sim = _guard(widget.engine.simulationStatus);
        _agent = _guard(widget.engine.agentStatus) ?? AgentStatus.off;
      });
      _frames = 0;
      _clock.reset();
    }
  }

  T? _guard<T>(T Function() call) {
    try {
      return call();
    } on ApiError catch (e) {
      _toast(context.error(e));
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

  /// The Polygon tool's corners so far (mm).
  final List<Offset> _polygon = [];

  /// Corners picked on the board (one, or two to measure between).
  List<PickedPoint> _points = const [];

  /// The Curve tool's handle being dragged: 1 or 2.
  int? _handle;

  void _setTool(String tool) => setState(() {
    _tool = tool;
    _sewFrom = null;
    _polygon.clear();
  });

  /// Board pick tolerance (logical pixels).
  static const _pickPx = 12.0;

  void _polygonTap(Offset mm, double mmPerPx) {
    if (_polygon.length >= 3 &&
        (mm - _polygon.first).distance < _pickPx * mmPerPx) {
      _finishPolygon();
      return;
    }
    setState(() => _polygon.add(mm));
  }

  void _finishPolygon() {
    if (_polygon.length < 3) return;
    final panels = (_summary['panels'] as List<dynamic>? ?? const []);
    final out = _run({
      'type': 'panel.create_polygon',
      'name': context.l.pieceNew(panels.length + 1),
      'points': [
        for (final p in _polygon) [p.dx, p.dy],
      ],
    });
    setState(_polygon.clear);
    _refresh();
    if (out != null) _toast(context.l.toastPieceAdded);
  }

  void _notchTap(Offset mm, double mmPerPx) {
    final notch = _guard(
      () => widget.engine.addNotchNear(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
      ),
    );
    if (notch != null) {
      _refresh();
      _toast(context.l.toastNotchAdded);
    }
  }

  /// The picked edge, as `panel.get` gives it.
  Map<String, dynamic>? get _pickedEdge => (_panel?['edges'] as List?)
      ?.cast<Map<String, dynamic>>()
      .where((e) => e['id'] == _edge)
      .firstOrNull;

  static Offset _at(Object? pos) =>
      Offset(((pos as List)[0] as num).toDouble(), (pos[1] as num).toDouble());

  /// Curve tool: pick an edge; a straight one gets handles on its line.
  void _curveTap(Offset mm, double mmPerPx) {
    _guard(
      () => widget.engine.pickEdgeNear(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
      ),
    );
    _refresh();
    final e = _pickedEdge;
    if (e == null || e['curve']['kind'] != 'line') return;
    final (a, b) = (_at(e['start']['pos']), _at(e['end']['pos']));
    final (c1, c2) = (a + (b - a) / 3, a + (b - a) * 2 / 3);
    _run({
      'type': 'edge.set_curve',
      'edge': e['id'],
      'curve': {
        'kind': 'cubic',
        'c1': [c1.dx, c1.dy],
        'c2': [c2.dx, c2.dy],
      },
    });
    _refresh();
  }

  bool _curvePress(Offset mm, double mmPerPx) {
    final e = _pickedEdge;
    if (e == null || e['curve']['kind'] != 'cubic') return false;
    for (final (k, key) in [(1, 'c1'), (2, 'c2')]) {
      if ((_at(e['curve'][key]) - mm).distance < _pickPx * mmPerPx) {
        _handle = k;
        return true;
      }
    }
    return false;
  }

  void _curveDrag(Offset mm) {
    final (e, h) = (_pickedEdge, _handle);
    if (e == null || h == null) return;
    final c1 = h == 1 ? mm : _at(e['curve']['c1']);
    final c2 = h == 2 ? mm : _at(e['curve']['c2']);
    _run({
      'type': 'edge.set_curve',
      'edge': e['id'],
      'curve': {
        'kind': 'cubic',
        'c1': [c1.dx, c1.dy],
        'c2': [c2.dx, c2.dy],
      },
    });
    _refresh();
  }

  /// What the board tools draw over the pattern.
  void _boardOverlay(
    Canvas canvas,
    Offset Function(Offset mm) toScreen,
    Palette p,
  ) {
    final line = Paint()
      ..color = p.accent
      ..strokeWidth = 1.5
      ..style = PaintingStyle.stroke;
    final dot = Paint()..color = p.accent;
    final ring = Paint()
      ..color = p.surface
      ..style = PaintingStyle.fill;
    void point(Offset at, double r) {
      canvas.drawCircle(at, r + 1.5, ring);
      canvas.drawCircle(at, r, dot);
    }

    if (_polygon.isNotEmpty) {
      final pts = [for (final q in _polygon) toScreen(q)];
      for (var i = 1; i < pts.length; i++) {
        canvas.drawLine(pts[i - 1], pts[i], line);
      }
      for (final (i, q) in pts.indexed) {
        point(q, i == 0 ? 6 : 4);
      }
    }
    final e = _tool == 'curve' ? _pickedEdge : null;
    if (e != null) {
      if (e['curve']['kind'] == 'cubic') {
        for (final (end, handle) in [
          (e['start']['pos'], e['curve']['c1']),
          (e['end']['pos'], e['curve']['c2']),
        ]) {
          final h = toScreen(_at(handle));
          canvas.drawLine(toScreen(_at(end)), h, line);
          point(h, 5);
        }
      }
    }
    if (_tool == 'select' && _points.isNotEmpty) {
      final pts = [for (final q in _points) toScreen(Offset(q.xMm, q.yMm))];
      if (pts.length == 2) {
        final (a, b) = (pts[0], pts[1]);
        final n = ((b - a).distance / 8).floor();
        for (var i = 0; i < n; i += 2) {
          canvas.drawLine(
            Offset.lerp(a, b, i / n)!,
            Offset.lerp(a, b, (i + 1) / n)!,
            line,
          );
        }
      }
      for (final q in pts) {
        point(q, 5);
      }
    }
  }

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
      _toast(context.l.toastSewn);
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
      'name': context.l.pieceNew(panels.length + 1),
      'origin': [right + 60, 0],
      'width': 300,
      'height': 400,
    });
    _refresh();
  }

  /// Ask where to save, then export there: the pattern, or the garment as
  /// the 3D view shows it.
  Future<void> _export(ExportFormat format, bool avatar) async {
    final ext = switch (format) {
      ExportFormat.svg => 'svg',
      ExportFormat.pdfA4 || ExportFormat.pdfSingle => 'pdf',
      ExportFormat.glb => 'glb',
      ExportFormat.obj => 'obj',
    };
    final name = (_summary['name'] as String?) ?? 'Rustle3D';
    final l = context.l;
    final location = await getSaveLocation(
      suggestedName: '$name.$ext',
      acceptedTypeGroups: [
        XTypeGroup(label: ext.toUpperCase(), extensions: [ext]),
      ],
    );
    if (location == null || !mounted) return;
    try {
      widget.engine.exportFile(
        format: format,
        path: location.path,
        avatar: avatar,
      );
      _toast(l.exported(location.path.split(RegExp(r'[/\\]')).last));
    } on ApiError catch (e) {
      _toast(context.error(e));
    }
  }

  void _allowAgents(bool allow) {
    _guard(
      () => widget.engine.allowAgents(
        libraryDir: widget.libraryDir,
        allow: allow,
      ),
    );
    setState(
      () => _agent = _guard(widget.engine.agentStatus) ?? AgentStatus.off,
    );
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
        title: Text(context.l.deleteTitle(panel['name'] as String)),
        content: Text(context.keys(context.l.deleteUndo)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(context.l.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(context.l.delete),
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
    return context.l.statsSim(
      s.particles,
      context.number(s.stepMs, 1),
      context.number(s.timeS, 1),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    return CallbackShortcuts(
      bindings: {
        // ⌘ on macOS, Ctrl elsewhere (where Ctrl+Y redoes too).
        SingleActivator(LogicalKeyboardKey.keyZ, meta: isMac, control: !isMac):
            _undo,
        SingleActivator(
          LogicalKeyboardKey.keyZ,
          meta: isMac,
          control: !isMac,
          shift: true,
        ): _redo,
        if (!isMac)
          const SingleActivator(LogicalKeyboardKey.keyY, control: true): _redo,
        const SingleActivator(LogicalKeyboardKey.keyR): _addRectangle,
        const SingleActivator(LogicalKeyboardKey.keyV): () =>
            _setTool('select'),
        const SingleActivator(LogicalKeyboardKey.keyD): () => _setTool('draw'),
        const SingleActivator(LogicalKeyboardKey.keyS): () => _setTool('sew'),
        const SingleActivator(LogicalKeyboardKey.keyP): () =>
            _setTool('polygon'),
        const SingleActivator(LogicalKeyboardKey.keyC): () => _setTool('curve'),
        const SingleActivator(LogicalKeyboardKey.keyN): () => _setTool('notch'),
        const SingleActivator(LogicalKeyboardKey.enter): _finishPolygon,
        const SingleActivator(LogicalKeyboardKey.escape): () =>
            setState(_polygon.clear),
        const SingleActivator(LogicalKeyboardKey.backspace): _deleteSelected,
        const SingleActivator(LogicalKeyboardKey.delete): _deleteSelected,
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
          key: _scaffold,
          endDrawer: LearnPanel(onShow: _showLearn),
          body: Column(
            children: [
              _TopBar(
                key: _tourTop,
                projectName: (_summary['name'] as String?) ?? '',
                dark: widget.dark,
                onToggleTheme: widget.onToggleTheme,
                language: widget.language,
                onLanguage: widget.onLanguage,
                onUndo: _undo,
                onRedo: _redo,
                onBody: _showBody,
                onExport: _export,
                agent: _agent,
                onAllowAgents: _allowAgents,
                onDisconnectAgent: () => _guard(widget.engine.disconnectAgent),
                bodyKey: _spotBody,
                onLearn: () => _scaffold.currentState?.openEndDrawer(),
                onHome: widget.onHome,
                step: _step,
                guide: _guide,
                onStep: _goTo,
              ),
              GuideBanner(
                key: _tourGuide,
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
                        keys: _spotTools,
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
                          key: _tourPattern,
                          title: l.viewPattern,
                          icon: Icons.architecture_outlined,
                          child: RustView(
                            engine: widget.engine,
                            kind: ViewKind.board,
                            // Corners and edges are dragged with Select.
                            holding: _tool == 'select',
                            pattern: switch (_tool) {
                              'polygon' => PatternPointer(onTap: _polygonTap),
                              'notch' => PatternPointer(onTap: _notchTap),
                              'curve' => PatternPointer(
                                onTap: _curveTap,
                                onPress: _curvePress,
                                onDrag: _curveDrag,
                                onRelease: () => _handle = null,
                              ),
                              _ => null,
                            },
                            overlay: (canvas, toScreen) => _boardOverlay(
                              canvas,
                              toScreen,
                              Tokens.of(context),
                            ),
                            onSelection: (_) =>
                                _tool == 'sew' ? _sewClick() : _refresh(),
                            onFrameTime: (ms) => _onFrame(true, ms),
                          ),
                        ),
                      ),
                      const SizedBox(width: Space.m),
                      Expanded(
                        child: _ViewCard(
                          key: _tourGarment,
                          title: '3D',
                          icon: Icons.view_in_ar_outlined,
                          trailing: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              _CameraBar(key: _spotCameras, onLook: _look),
                              const SizedBox(width: Space.s),
                              _SimControls(
                                key: _spotSimulate,
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
                              _toast(l.toastPinned);
                            },
                            drawing: _tool == 'draw',
                            onDrawn: (_) {
                              _refresh();
                              _toast(l.toastLineAdded);
                            },
                          ),
                        ),
                      ),
                      const SizedBox(width: Space.m),
                      SizedBox(
                        width: 264,
                        child: _Inspector(
                          key: _spotInspector,
                          selection: _selection,
                          points: _points,
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
                            // A new placement starts the drape again from
                            // around the body.
                            if (step['type'] == 'panel.place') _restart();
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
                  'sew' when _sewFrom == null => l.hintSewFirst,
                  'sew' => l.hintSewSecond,
                  'draw' => l.hintDraw,
                  'polygon' => l.hintPolygon,
                  'curve' => l.hintCurve,
                  'notch' => l.hintNotch,
                  _ when _selection.isEmpty => context.keys(l.hintIdle),
                  _ => context.keys(l.hintSelection),
                },
                stats:
                    _simStats() +
                    l.statsRender(
                      context.number(_boardMs, 1),
                      context.number(_garmentMs, 1),
                      context.number(_fps),
                    ),
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
    super.key,
    required this.projectName,
    required this.dark,
    required this.onToggleTheme,
    required this.language,
    required this.onLanguage,
    required this.onUndo,
    required this.onRedo,
    required this.onBody,
    required this.onExport,
    required this.agent,
    required this.onAllowAgents,
    required this.onDisconnectAgent,
    required this.bodyKey,
    required this.onLearn,
    required this.onHome,
    required this.step,
    required this.guide,
    required this.onStep,
  });

  final String projectName;
  final bool dark;
  final VoidCallback onToggleTheme;
  final String language;
  final ValueChanged<String> onLanguage;
  final VoidCallback onUndo;
  final VoidCallback onRedo;
  final VoidCallback onBody;

  /// Export in a format (with the avatar, for 3D formats).
  final void Function(ExportFormat format, bool avatar) onExport;

  /// AI agents: allowed or not, connected or not.
  final AgentStatus agent;
  final ValueChanged<bool> onAllowAgents;
  final VoidCallback onDisconnectAgent;

  /// The Body button, for Learn to point at.
  final GlobalKey bodyKey;
  final VoidCallback onLearn;
  final VoidCallback onHome;
  final GuideStep step;

  /// The guide is open: the current step is shown as active.
  final bool guide;
  final ValueChanged<GuideStep> onStep;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    const steps = GuideStep.values;
    return SizedBox(
      height: 56,
      child: Padding(
        // Leave room for the macOS window buttons (transparent title bar).
        padding: const EdgeInsets.only(left: 84, right: Space.m),
        child: Row(
          children: [
            Tooltip(
              message: l.allProjects,
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
            // The steps take the room left, and scroll when the window is
            // too narrow for them.
            Expanded(
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    for (var i = 0; i < steps.length; i++) ...[
                      _StepChip(
                        index: i + 1,
                        label: steps[i].label(l),
                        active: guide && steps[i] == step,
                        done: steps[i].index < step.index,
                        onTap: () => onStep(steps[i]),
                      ),
                      if (i < steps.length - 1)
                        Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: Space.xs,
                          ),
                          child: Icon(
                            Icons.chevron_right,
                            size: 16,
                            color: p.textMuted,
                          ),
                        ),
                    ],
                  ],
                ),
              ),
            ),
            const SizedBox(width: Space.s),
            TextButton.icon(
              onPressed: onLearn,
              icon: const Icon(Icons.school_outlined, size: 18),
              label: Text(l.learn),
            ),
            TextButton.icon(
              key: bodyKey,
              onPressed: onBody,
              icon: const Icon(Icons.accessibility_new_rounded, size: 18),
              label: Text(l.body),
            ),
            _AgentButton(
              status: agent,
              onAllow: onAllowAgents,
              onDisconnect: onDisconnectAgent,
            ),
            PopupMenuButton<(ExportFormat, bool)>(
              tooltip: l.export,
              onSelected: (choice) => onExport(choice.$1, choice.$2),
              itemBuilder: (_) => [
                for (final (format, avatar, label) in [
                  (ExportFormat.svg, false, l.exportSvg),
                  (ExportFormat.pdfA4, false, l.exportPdfA4),
                  (ExportFormat.pdfSingle, false, l.exportPdfSingle),
                  (ExportFormat.glb, false, l.exportGlb),
                  (ExportFormat.glb, true, l.exportGlbAvatar),
                  (ExportFormat.obj, false, l.exportObj),
                ])
                  PopupMenuItem(value: (format, avatar), child: Text(label)),
              ],
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.s,
                  vertical: 6,
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.ios_share_rounded, size: 18, color: p.accent),
                    const SizedBox(width: 6),
                    Text(
                      l.export,
                      style: ts(
                        fontSize: 14,
                        fontWeight: FontWeight.w500,
                        color: p.accent,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(width: Space.s),
            _IconAction(
              icon: Icons.undo_rounded,
              tip: context.keys(l.undoTip),
              onTap: onUndo,
            ),
            _IconAction(
              icon: Icons.redo_rounded,
              tip: context.keys(l.redoTip),
              onTap: onRedo,
            ),
            const SizedBox(width: Space.s),
            LanguageButton(current: language, onChanged: onLanguage),
            _IconAction(
              icon: dark ? Icons.light_mode_outlined : Icons.dark_mode_outlined,
              tip: dark ? l.themeLight : l.themeDark,
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
    required this.keys,
    required this.onSelect,
  });

  final String active;

  /// Buttons Learn points at, by tool.
  final Map<String, GlobalKey> keys;

  /// The tool the guided flow's current step uses, ringed.
  final String? highlight;
  final ValueChanged<String> onSelect;

  /// id, icon, name, key, hint.
  static List<(String, IconData, String, String, String)> _tools(L l) => [
    ('select', Icons.near_me_outlined, l.toolSelect, 'V', l.toolSelectHint),
    (
      'rectangle',
      Icons.crop_square_rounded,
      l.toolRectangle,
      'R',
      l.toolRectangleHint,
    ),
    ('polygon', Icons.pentagon_outlined, l.toolPolygon, 'P', l.toolPolygonHint),
    ('curve', Icons.gesture_rounded, l.toolCurve, 'C', l.toolCurveHint),
    ('draw', Icons.brush_rounded, l.toolDraw, 'D', l.toolDrawHint),
    (
      'notch',
      Icons.vertical_align_center_rounded,
      l.toolNotch,
      'N',
      l.toolNotchHint,
    ),
    ('sew', Icons.link_rounded, l.toolSew, 'S', l.toolSewHint),
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
          for (final (id, icon, name, key, hint) in _tools(context.l))
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
                  key: keys[id],
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
    super.key,
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
    super.key,
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
    super.key,
    required this.selection,
    required this.points,
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
  final List<PickedPoint> points;
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
              points: points,
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
    final l = context.l;
    final count = (summary['counts']?['panels'] as int?) ?? 0;
    return ListView(
      children: [
        Text(
          l.inspector,
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
            l.selectPanel,
            style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
          ),
        ),
        const SizedBox(height: Space.xs),
        Center(
          child: Text(
            l.selectPanelHint(count),
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
          label: Text(l.arrange),
        ),
        const SizedBox(height: Space.xs),
        Text(
          l.arrangeHint,
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
          context.l.pins,
          style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Text(
          context.keys(pins.isEmpty ? context.l.pinsEmpty : context.l.pinsAdd),
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
                  tip: context.l.removePin,
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
          context.l.wholeGarment,
          style: ts(fontSize: 15, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.s),
        Text(
          context.l.piecesSelected(count),
          style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
        ),
      ],
    );
  }
}

class _PanelInspector extends StatelessWidget {
  const _PanelInspector({
    required this.panel,
    required this.points,
    required this.edge,
    required this.seams,
    required this.run,
    required this.onDelete,
  });

  final Map<String, dynamic> panel;
  final List<PickedPoint> points;
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
    final l = context.l;
    return ListView(
      children: [
        Text(l.piece, style: ts(fontSize: 12, color: p.textMuted)),
        const SizedBox(height: Space.xs),
        Text(
          panel['name'] as String,
          style: ts(fontSize: 24, fontWeight: FontWeight.w600, color: p.text),
        ),
        if (linked) ...[
          const SizedBox(height: Space.s),
          _Badge(
            icon: Icons.flip_rounded,
            text: l.mirrorCopy,
            action: l.unlink,
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
        if (points.isNotEmpty) ...[
          const SizedBox(height: Space.m),
          _PointsCard(points: points, run: run),
        ],
        const SizedBox(height: Space.l),
        Row(
          children: [
            Expanded(
              child: _Stat(
                label: l.area,
                value: '${context.number(area, 3)} m²',
              ),
            ),
            const SizedBox(width: Space.s),
            Expanded(
              child: _Stat(
                label: l.perimeter,
                value: '${context.number(perimeter, 1)} ${l.cm}',
              ),
            ),
          ],
        ),
        const SizedBox(height: Space.xl),
        _SliderRow(
          label: l.seamAllowance,
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
          label: l.grainline,
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
        _PlacementCard(
          placement: panel['placement'] as Map<String, dynamic>?,
          place: (place) => run({'type': 'panel.place', 'panel': id, ...place}),
        ),
        const SizedBox(height: Space.xl),
        Row(
          children: [
            Expanded(
              child: OutlinedButton.icon(
                onPressed: linked
                    ? null
                    : () => run({
                        'type': 'panel.mirror',
                        'panel': id,
                        'name': l.pieceMirror(panel['name'] as String),
                      }),
                icon: const Icon(Icons.flip_rounded, size: 16),
                label: Text(l.mirror),
              ),
            ),
            const SizedBox(width: Space.s),
            Expanded(
              child: OutlinedButton.icon(
                onPressed: onDelete,
                style: OutlinedButton.styleFrom(foregroundColor: p.danger),
                icon: const Icon(Icons.delete_outline_rounded, size: 16),
                label: Text(l.delete),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

/// Where the piece sits for the simulation: around the body, an arm, or
/// flat; at which angle and height.
class _PlacementCard extends StatelessWidget {
  const _PlacementCard({required this.placement, required this.place});

  final Map<String, dynamic>? placement;

  /// Run `panel.place` with these fields.
  final ValueChanged<Map<String, Object?>> place;

  static List<(String, String)> _around(L l) => [
    ('body', l.aroundBody),
    ('left_arm', l.aroundLeftArm),
    ('right_arm', l.aroundRightArm),
    ('flat', l.aroundFlat),
  ];

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final current = placement;
    final l = context.l;
    final header = Text(
      l.onTheBody,
      style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
    );
    if (current == null) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          header,
          const SizedBox(height: Space.xs),
          Text(l.notSimulated, style: ts(fontSize: 12, color: p.textMuted)),
          const SizedBox(height: Space.s),
          OutlinedButton.icon(
            onPressed: () =>
                place({'around': 'body', 'angle_deg': 0, 'top_cm': 110}),
            icon: const Icon(Icons.accessibility_new_rounded, size: 16),
            label: Text(l.placeOnBody),
          ),
        ],
      );
    }
    final around = current['around'] as String;
    final angle = (current['angle_deg'] as num).toDouble();
    final top = (current['top_cm'] as num).toDouble();
    final offset = (current['offset_mm'] as num).toDouble();
    // Flat pieces are measured from the body's axis, the others from its
    // surface: switching between them takes a sensible distance.
    void update({String? to, double? angleDeg, double? topCm}) {
      final next = to ?? around;
      place({
        'around': next,
        'angle_deg': (angleDeg ?? angle).roundToDouble(),
        'top_cm': (topCm ?? top).roundToDouble(),
        'offset_mm': (next == 'flat') == (around == 'flat')
            ? offset
            : (next == 'flat' ? 420 : 30),
      });
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        header,
        const SizedBox(height: Space.s),
        // A menu rather than segments: the names fit in any language.
        DropdownButtonFormField<String>(
          initialValue: around,
          isDense: true,
          decoration: const InputDecoration(
            border: OutlineInputBorder(),
            isDense: true,
          ),
          items: [
            for (final (value, label) in _around(l))
              DropdownMenuItem(value: value, child: Text(label)),
          ],
          onChanged: (to) => update(to: to),
        ),
        const SizedBox(height: Space.l),
        _SliderRow(
          label: around == 'body' || around == 'flat'
              ? l.aroundAngle
              : l.aroundArmAngle,
          value: angle.clamp(0, 359),
          min: 0,
          max: 359,
          unit: '°',
          enabled: true,
          onChanged: (v) => update(angleDeg: v),
        ),
        const SizedBox(height: Space.l),
        _SliderRow(
          label: l.topHeight,
          value: top.clamp(40, 200),
          min: 40,
          max: 200,
          unit: l.cm,
          enabled: true,
          onChanged: (v) => update(topCm: v),
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

/// A slider whose value can also be typed.
class _SliderRow extends StatefulWidget {
  const _SliderRow({
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.unit,
    required this.enabled,
    required this.onChanged,
    this.digits = 0,
    this.typedMin,
    this.typedMax,
    this.fieldKey,
  });

  final String label;
  final double value;
  final double min;
  final double max;
  final String unit;
  final bool enabled;
  final ValueChanged<double> onChanged;

  /// Decimals shown and kept.
  final int digits;

  /// What can be typed, if wider than the slider's range.
  final double? typedMin;
  final double? typedMax;
  final Key? fieldKey;

  @override
  State<_SliderRow> createState() => _SliderRowState();
}

class _SliderRowState extends State<_SliderRow> {
  late final _text = TextEditingController(text: _shown(widget.value));
  final _focus = FocusNode();

  String _shown(double v) => context.decimal(v, widget.digits);

  @override
  void initState() {
    super.initState();
    // Leaving the field applies what was typed.
    _focus.addListener(() {
      if (!_focus.hasFocus) _submit();
    });
  }

  @override
  void didUpdateWidget(_SliderRow old) {
    super.didUpdateWidget(old);
    if (!_focus.hasFocus) _text.text = _shown(widget.value);
  }

  @override
  void dispose() {
    _text.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _submit() {
    final typed = _parseNumber(_text.text);
    if (typed == null) {
      _text.text = _shown(widget.value);
      return;
    }
    final v = typed
        .clamp(widget.typedMin ?? widget.min, widget.typedMax ?? widget.max)
        .toDouble();
    _text.text = _shown(v);
    if (v != widget.value) widget.onChanged(v);
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                widget.label,
                style: ts(
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                  color: p.text,
                ),
              ),
            ),
            SizedBox(
              width: 76,
              child: _NumberField(
                key: widget.fieldKey,
                controller: _text,
                focus: _focus,
                unit: widget.unit,
                enabled: widget.enabled,
                onSubmitted: _submit,
              ),
            ),
          ],
        ),
        Slider(
          value: widget.value.clamp(widget.min, widget.max),
          min: widget.min,
          max: widget.max,
          onChanged: widget.enabled ? widget.onChanged : null,
        ),
      ],
    );
  }
}

/// A number typed by the user, with a decimal point or comma.
double? _parseNumber(String text) =>
    double.tryParse(text.trim().replaceAll(',', '.'));

/// A compact field for a number, with its unit after it.
class _NumberField extends StatelessWidget {
  const _NumberField({
    super.key,
    required this.controller,
    required this.unit,
    this.onSubmitted,
    this.focus,
    this.enabled = true,
  });

  final TextEditingController controller;
  final FocusNode? focus;
  final String unit;
  final bool enabled;
  final VoidCallback? onSubmitted;

  /// Typed characters go to the field, not to the window's one-key
  /// shortcuts (1–4 would turn the camera, R add a piece).
  static KeyEventResult _keepTyping(FocusNode _, KeyEvent event) {
    final keys = HardwareKeyboard.instance;
    return event.character != null &&
            !keys.isMetaPressed &&
            !keys.isControlPressed
        ? KeyEventResult.skipRemainingHandlers
        : KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Focus(
      canRequestFocus: false,
      skipTraversal: true,
      onKeyEvent: _keepTyping,
      child: TextField(
        controller: controller,
        focusNode: focus,
        enabled: enabled,
        textAlign: TextAlign.right,
        style: ts(fontSize: 13, color: p.text, tabular: true),
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        inputFormatters: [
          FilteringTextInputFormatter.allow(RegExp(r'[0-9.,]')),
        ],
        onSubmitted: (_) => onSubmitted?.call(),
        decoration: InputDecoration(
          isDense: true,
          suffixText: unit,
          suffixStyle: ts(fontSize: 12, color: p.textMuted),
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 8,
            vertical: 7,
          ),
          border: const OutlineInputBorder(),
        ),
      ),
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
  const _CameraBar({super.key, required this.onLook});

  final ValueChanged<CameraSide> onLook;

  static List<(CameraSide, String, String)> _sides(L l) => [
    (CameraSide.front, l.camFront, l.camFrontTip),
    (CameraSide.left, l.camSide, l.camSideTip),
    (CameraSide.back, l.camBack, l.camBackTip),
    (CameraSide.threeQuarter, '¾', l.camThreeQuarterTip),
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
          for (final (side, label, tip) in _sides(context.l))
            Tooltip(
              message: tip,
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
    super.key,
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
            message: running ? context.l.pauseTip : context.l.simulateTip,
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
              label: Text(running ? context.l.pause : context.l.simulate),
            ),
          ),
          const SizedBox(width: 2),
          _IconAction(
            icon: Icons.restart_alt_rounded,
            tip: context.l.restartTip,
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

  /// A fabric's name and blurb.
  static (String, String) _text(L l, String id) => switch (id) {
    'cotton' => (l.fabricCotton, l.fabricCottonBlurb),
    'denim' => (l.fabricDenim, l.fabricDenimBlurb),
    'jersey' => (l.fabricJersey, l.fabricJerseyBlurb),
    'silk' => (l.fabricSilk, l.fabricSilkBlurb),
    'wool' => (l.fabricWool, l.fabricWoolBlurb),
    _ => (id, ''),
  };

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final swatches = Engine.fabrics();
    final l = context.l;
    final (name, blurb) = _text(l, current);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l.fabric,
          style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Text('$name · $blurb', style: ts(fontSize: 12.5, color: p.textMuted)),
        const SizedBox(height: Space.m),
        Wrap(
          spacing: Space.s,
          runSpacing: Space.s,
          children: [
            for (final f in swatches)
              Tooltip(
                message: _text(l, f.id).$1,
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
    final l = context.l;
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
                l.edgeLength(context.number(length, 1)),
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
            context.keys(
              other == null
                  ? l.edgeOpen
                  : l.edgeSewn(
                      '${other['panel_name'] ?? other['panel']}',
                      context.number(seam['mismatch_mm'] as num, 1),
                    ),
            ),
            style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
          ),
          const SizedBox(height: Space.m),
          _LengthControl(
            key: ValueKey(id),
            label: l.edgeLengthLabel,
            fieldKey: const Key('edge-length'),
            cm: length,
            onCm: (cm) => run({
              'type': 'edge.set_length',
              'edge': id,
              'length_mm': cm * 10,
            }),
          ),
          if (edge['curve']?['kind'] == 'cubic')
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                onPressed: () => run({
                  'type': 'edge.set_curve',
                  'edge': id,
                  'curve': {'kind': 'line'},
                }),
                icon: const Icon(Icons.horizontal_rule_rounded, size: 16),
                label: Text(l.straighten),
              ),
            ),
          if (seam != null)
            Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        l.gathered,
                        style: ts(fontSize: 13, fontWeight: FontWeight.w500),
                      ),
                      if (seam['gathered'] == true)
                        Text(
                          l.gatheredHint(
                            context.number(seam['gathering_ratio'] as num, 1),
                          ),
                          style: ts(fontSize: 12, color: p.textMuted),
                        ),
                    ],
                  ),
                ),
                Switch(
                  value: seam['gathered'] == true,
                  onChanged: (on) => run({
                    'type': 'seam.set_gathered',
                    'seam': seam['id'],
                    'gathered': on,
                  }),
                ),
              ],
            ),
          if (seam != null &&
              seam['gathered'] != true &&
              (seam['mismatch_mm'] as num) >= 0.5)
            Padding(
              padding: const EdgeInsets.only(top: Space.xs),
              child: OutlinedButton.icon(
                onPressed: () => run({
                  'type': 'seam.match_length',
                  'seam': seam['id'],
                  'keep': id,
                }),
                icon: const Icon(Icons.straighten_rounded, size: 16),
                label: Text(l.matchOtherSide),
              ),
            ),
          if (seam != null)
            Wrap(
              children: [
                TextButton(
                  onPressed: () =>
                      run({'type': 'seam.flip', 'seam': seam['id']}),
                  child: Text(l.flipSeam),
                ),
                TextButton(
                  onPressed: () =>
                      run({'type': 'seam.remove', 'seam': seam['id']}),
                  child: Text(l.unpick),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

/// A length, typed or set with a slider (half to one and a half times
/// what it was when picked).
class _LengthControl extends StatefulWidget {
  const _LengthControl({
    super.key,
    required this.label,
    required this.cm,
    required this.onCm,
    this.fieldKey,
  });

  final String label;
  final double cm;
  final ValueChanged<double> onCm;
  final Key? fieldKey;

  @override
  State<_LengthControl> createState() => _LengthControlState();
}

class _LengthControlState extends State<_LengthControl> {
  /// The slider's range stays put while it is dragged.
  late final double _picked = widget.cm;

  @override
  Widget build(BuildContext context) => _SliderRow(
    label: widget.label,
    value: widget.cm,
    min: (_picked * 0.5).floorToDouble(),
    max: (_picked * 1.5).ceilToDouble(),
    typedMin: 0.1,
    typedMax: 1000,
    unit: context.l.cm,
    digits: 1,
    enabled: true,
    fieldKey: widget.fieldKey,
    onChanged: (cm) => widget.onCm((cm * 10).roundToDouble() / 10),
  );
}

/// AI agents: let one connect (`r3d mcp --attach`), see whether one is,
/// disconnect it.
class _AgentButton extends StatelessWidget {
  const _AgentButton({
    required this.status,
    required this.onAllow,
    required this.onDisconnect,
  });

  final AgentStatus status;
  final ValueChanged<bool> onAllow;
  final VoidCallback onDisconnect;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final connected = status == AgentStatus.connected;
    final label = switch (status) {
      AgentStatus.off => l.agents,
      AgentStatus.waiting => l.agentsWaiting,
      AgentStatus.connected => l.agentConnected,
    };
    final color = status == AgentStatus.off ? p.textMuted : p.accent;
    return PopupMenuButton<String>(
      tooltip: l.agentsTip,
      onSelected: (choice) => switch (choice) {
        'allow' => onAllow(status == AgentStatus.off),
        'disconnect' => onDisconnect(),
        _ => null,
      },
      itemBuilder: (_) => [
        CheckedPopupMenuItem(
          value: 'allow',
          checked: status != AgentStatus.off,
          child: Text(l.allowAgents),
        ),
        PopupMenuItem(
          enabled: false,
          child: Text(
            l.agentsHow,
            style: ts(fontSize: 12.5, color: p.textMuted),
          ),
        ),
        if (connected)
          PopupMenuItem(value: 'disconnect', child: Text(l.disconnectAgent)),
      ],
      child: AnimatedContainer(
        duration: Motion.fast,
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
        decoration: BoxDecoration(
          color: connected ? p.accentSoft : Colors.transparent,
          borderRadius: BorderRadius.circular(999),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.smart_toy_outlined, size: 18, color: color),
            const SizedBox(width: 6),
            Text(
              label,
              style: ts(
                fontSize: 13,
                fontWeight: FontWeight.w500,
                color: color,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// The corners picked on the board: with two, their distance, typed or
/// with a slider (the second one moves along the line).
class _PointsCard extends StatelessWidget {
  const _PointsCard({required this.points, required this.run});

  final List<PickedPoint> points;
  final ValueChanged<Map<String, Object?>> run;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final pair = points.length == 2 ? (points[0], points[1]) : null;
    return Container(
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        color: p.accentSoft,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            context.keys(pair == null ? l.pointPicked : l.pointsPicked),
            style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
          ),
          if (pair case (final a, final b)) ...[
            const SizedBox(height: Space.m),
            _LengthControl(
              key: ValueKey('${a.id}-${b.id}'),
              label: l.distance,
              fieldKey: const Key('distance'),
              cm: (Offset(b.xMm, b.yMm) - Offset(a.xMm, a.yMm)).distance / 10,
              onCm: (cm) => run({
                'type': 'point.set_distance',
                'from': a.id,
                'point': b.id,
                'distance_mm': cm * 10,
              }),
            ),
          ],
        ],
      ),
    );
  }
}
