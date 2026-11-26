// The main window: top bar, tool rail, the 2D board and the 3D view side by
// side, the inspector and the status bar (docs/ux.md §4).

import 'dart:async';
import 'dart:convert';
import 'dart:math' show atan2, max, min, pi;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'agents.dart';
import 'body_check.dart';
import 'body_panel.dart';
import 'busy.dart';
import 'design/tokens.dart';
import 'i18n.dart';
import 'learn.dart';
import 'motion_library.dart';
import 'pattern_library.dart';
import 'photo.dart';
import 'projector.dart';
import 'techpack.dart';
import 'tour.dart';
import 'rust/api/engine.dart';
import 'rust/api/library.dart';
import 'start_screen.dart' show Logo;
import 'views/rust_view.dart';

part 'marker.dart';
part 'game_export.dart';
part 'fit.dart';
part 'sketch.dart';
part 'action_bar.dart';

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

  /// The open project. The shell owns it: it disposes it when it goes,
  /// after its views (they leave first, at the end of the transition
  /// back to the start screen).
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

  /// The imported body's check (`avatar.check`); `null` with the
  /// built-in body.
  Map<String, dynamic>? _bodyCheck;

  /// The imported body's arrangement points are drawn in the 3D view.
  bool _showPoints = false;
  List<String> _selection = const [];
  Map<String, dynamic>? _panel;
  List<dynamic> _pins = const [];
  List<dynamic> _seams = const [];
  String? _edge;

  /// The Button tool's first click: the piece and the point (mm) where
  /// the button goes.
  (String, Offset)? _buttonFirst;

  /// The Zipper tool's first click: the edge, and whether its bottom
  /// stop is at its end (the click was nearer the end).
  (String, bool)? _zipFirst;

  /// What is picked on the board besides corners: edges, notches (with
  /// where they are) and construction lines.
  List<String> _edges = const [];
  List<PickedPoint> _notches = const [];
  List<String> _guides = const [];

  /// Every construction line (`guide.list`), to show those picked.
  List<dynamic> _guideList = const [];

  /// Every piece's constraints (`constraint.list`), drawn on the board;
  /// the selected piece's, and how constrained it is, in the inspector.
  List<dynamic> _allConstraints = const [];
  List<dynamic> _constraints = const [];
  List<dynamic> _constraintGroups = const [];

  /// A dimension just added from the action bar: its value field opens
  /// where its label is drawn.
  String? _fieldFor;

  /// Where each dimension's label was drawn (board pixels).
  final List<(Map<String, dynamic>, Rect)> _labelRects = [];

  /// A label being dragged: the dimension, its offset when pressed (mm),
  /// where it was pressed (mm), and whether it moved.
  (Map<String, dynamic>, Offset, Offset, bool)? _labelDrag;
  String _tool = 'select';

  /// The undo history: every step's command name, oldest first, and how
  /// many are done.
  List<String> _history = const [];
  int _historyAt = 0;

  // Performance readouts, smoothed and refreshed twice a second.
  double _boardMs = 0;
  double _garmentMs = 0;
  int _frames = 0;
  double _fps = 0;
  SimStatus? _sim;

  /// The photo shown in the 3D view (while it refines, and after).
  PhotoStatus? _photo;

  /// How the last photo was asked for.
  PhotoChoice _photoChoice = const PhotoChoice();
  final _clock = Stopwatch()..start();

  /// AI agents: whether they may connect, and the changes they made (the
  /// views refresh when it moves).
  AgentStatus _agent = AgentStatus.off;
  BigInt _agentChanges = BigInt.zero;

  /// Asks for both several times a second: an agent connects and works
  /// while the views may be still (its long actions hold the project, so
  /// nothing redraws).
  Timer? _agentPoll;

  /// The view taking the space of both views, if one does.
  ViewKind? _maximized;

  /// The drape was paused because the 3D view went out of sight: it goes
  /// on when the view comes back.
  bool _resumeDrape = false;

  /// The view under the pointer, which the shortcut maximizes.
  ViewKind _hovered = ViewKind.board;

  /// Where the window's one-key shortcuts listen: a press on a view gives
  /// the keys back to them (after typing in a field, say).
  final _keys = FocusNode(debugLabel: 'shortcuts');

  /// When nothing in the window has the keys any more (a field let them
  /// go, or was removed while typing in it), they go back to the tools.
  void _keepKeys() {
    final focus = FocusManager.instance.primaryFocus;
    if (focus != null && focus == _keys.enclosingScope) _keys.requestFocus();
  }

  /// The body is being fitted to new measurements (the 3D view shows the
  /// old one meanwhile).
  bool _fitting = false;

  /// The fit map colouring the cloth (`strain`, `gap`), its legend, and
  /// the timer that refreshes the legend's tightest value.
  String? _fitMap;
  FitLegend? _fitLegend;
  Timer? _fitPoll;

  /// The tape measure's last reading round the cloth (Tape measure tool).
  TapeReading? _tape;

  /// The tape measures a length between two points, not round.
  bool _tapeLength = false;

  /// The length's first point (m), while the second is awaited.
  List<double>? _lengthFrom;

  /// The last length measured.
  LengthReading? _length;

  @override
  void initState() {
    super.initState();
    FocusManager.instance.addListener(_keepKeys);
    // Saved patterns, for the app's and attached agents' library steps.
    widget.engine.setLibrary(libraryDir: widget.libraryDir);
    _refresh();
    _agentPoll = Timer.periodic(
      const Duration(milliseconds: 200),
      (_) => _pollAgents(),
    );
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _maybeTour();
      // The garment placed around the body, ready to simulate (the body is
      // fitted first: seconds on the first project opened).
      if (!(_guard(widget.engine.simulationStatus)?.loaded ?? true)) {
        _drape(widget.engine.resetSimulation, title: context.l.busyOpening);
      }
    });
  }

  @override
  void dispose() {
    FocusManager.instance.removeListener(_keepKeys);
    _keys.dispose();
    _agentPoll?.cancel();
    _fitPoll?.cancel();
    widget.engine.dispose();
    super.dispose();
  }

  /// An agent's change: show it at once; and whether one is connected.
  void _pollAgents() {
    if (!mounted) return;
    final changes = _guard(widget.engine.agentChanges);
    if (changes != null && changes != _agentChanges) {
      _agentChanges = changes;
      _refresh();
    }
    final agent = _guard(widget.engine.agentStatus) ?? AgentStatus.off;
    if (agent != _agent) setState(() => _agent = agent);
    final fitting = _guard(widget.engine.bodyFitting) ?? false;
    if (fitting != _fitting) setState(() => _fitting = fitting);
    // An agent may start the drape while the 3D view is out of sight.
    if (_maximized == ViewKind.board) _pauseHidden();
  }

  /// Let `view` take the space of both views, or (`null`) show both.
  void _maximize(ViewKind? view) {
    if (view == _maximized) return;
    setState(() => _maximized = view);
    if (view == ViewKind.board) {
      _pauseHidden();
    } else if (_resumeDrape) {
      _resumeDrape = false;
      _setRunning(true);
    }
  }

  void _toggleMaximized(ViewKind view) =>
      _maximize(_maximized == null ? view : null);

  /// Pause a running drape while the 3D view is hidden; it goes on when
  /// the view comes back.
  void _pauseHidden() {
    final status = _guard(widget.engine.simulationStatus);
    if (status == null) return;
    if (status.running) {
      _resumeDrape = true;
      _drape(() => widget.engine.runSimulation(running: false));
    } else if (status != _sim) {
      setState(() => _sim = status);
    }
  }

  void _showBody() =>
      showBodyPanel(context, widget.engine, widget.libraryDir, _refresh);

  void _showBodyCheck() => showBodyCheck(context, widget.engine, _refresh);

  /// "The body has things to check", with a way to the check.
  void _bodyNotice() {
    final l = context.l;
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(l.bodyHasWarnings),
          behavior: SnackBarBehavior.floating,
          action: SnackBarAction(
            label: l.bodyMenuCheck,
            onPressed: _showBodyCheck,
          ),
        ),
      );
  }

  /// The Body menu: `measurements`, `import`, `check`, `points` (show or
  /// hide the arrangement points) or `builtin`.
  void _onBodyMenu(String choice) {
    switch (choice) {
      case 'measurements':
        _showBody();
      case 'import':
        _importBody();
      case 'check':
        _showBodyCheck();
      case 'points':
        setState(() => _showPoints = !_showPoints);
      case 'builtin':
        _builtinBody();
    }
  }

  /// Ask for a body file (OBJ, glTF, GLB, PLY, STL), import it, show its check.
  Future<void> _importBody() async {
    final l = context.l;
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(
          label: l.bodyFiles,
          extensions: const ['obj', 'gltf', 'glb', 'ply', 'stl'],
        ),
      ],
    );
    if (file == null || !mounted) return;
    try {
      await runBusy(
        context,
        widget.engine,
        l.bodyMenuImport,
        () => widget.engine.importBody(path: file.path),
      );
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
      return;
    }
    if (!mounted) return;
    _refresh();
    _toast(l.bodyImported);
    _showBodyCheck();
  }

  /// Drape on the built-in body again.
  Future<void> _builtinBody() async {
    try {
      await runBusy(
        context,
        widget.engine,
        context.l.busyBodyTitle,
        () => widget.engine.runLong(stepJson: '{"type":"avatar.remove_mesh"}'),
      );
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
      return;
    }
    if (!mounted) return;
    _refresh();
    _toast(context.l.bodyBuiltinBack);
  }

  final _scaffold = GlobalKey<ScaffoldState>();

  // What the first-launch tour points at.
  final _tourTop = GlobalKey();
  final _tourPattern = GlobalKey();
  final _tourGarment = GlobalKey();

  // What Learn cards point at, besides the tour's areas.
  final _spotInspector = GlobalKey();
  final _spotSimulate = GlobalKey();
  final _spotPhoto = GlobalKey();
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
    // The marks point at both views.
    if (_maximized != null) {
      _maximize(null);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _showLearn(stops);
      });
      return;
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
            Spot.photo => _spotPhoto,
          },
          title,
          text,
        ),
    ], done: context.l.gotIt);
  }

  void _look(CameraSide side) =>
      _guard(() => widget.engine.lookFrom(side: side));

  /// Steps that can take seconds: drafting a template or a size range in
  /// every size, fitting a body to new measurements.
  static const _longSteps = {
    'template.apply',
    'library.add',
    'grading.use_chart',
    'grading.set_sizes',
    'grading.show_size',
    'avatar.set',
    'garment.redraft',
  };

  /// Run a step that can take seconds off the UI thread, showing its
  /// progress; the user may stop it (nothing changes then).
  Future<dynamic> _runLong(Map<String, Object?> step) async {
    final l = context.l;
    final title = switch (step['type']) {
      'template.apply' => l.busyTemplate,
      'library.add' => l.busyLibraryAdd,
      'avatar.set' => l.busyBodyTitle,
      'garment.redraft' => l.busyRedraft,
      _ => l.busySizesTitle,
    };
    try {
      final out = await runBusy(
        context,
        widget.engine,
        title,
        () => widget.engine.runLong(stepJson: jsonEncode(step)),
        cancellable: true,
      );
      return jsonDecode(out);
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
      return null;
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
      try {
        _bodyCheck = jsonDecode(
          widget.engine.run(stepJson: '{"type":"avatar.check"}'),
        ) as Map<String, dynamic>;
      } on ApiError {
        // The built-in body: nothing to check.
        _bodyCheck = null;
        _showPoints = false;
      }
      _selection = widget.engine.selection();
      _panel = _selection.length == 1
          ? _run({'type': 'panel.get', 'panel': _selection.first})
                as Map<String, dynamic>?
          : null;
      _pins = (_run({'type': 'pin.list'}) as List<dynamic>?) ?? const [];
      _seams = (_run({'type': 'seam.list'}) as List<dynamic>?) ?? const [];
      _edge = _guard<String?>(widget.engine.selectedEdge);
      _edges = _guard(widget.engine.selectedEdges) ?? const [];
      _guides = _guard(widget.engine.selectedGuides) ?? const [];
      _notches = _guard(widget.engine.selectedNotches) ?? const [];
      _points = _guard(widget.engine.selectedPoints) ?? const [];
      _guideList = (_run({'type': 'guide.list'}) as List?) ?? const [];
      final constraints =
          _run({'type': 'constraint.list'}) as Map<String, dynamic>?;
      _allConstraints = (constraints?['constraints'] as List?) ?? const [];
      final piece = _selection.length == 1 ? _selection.first : null;
      _constraints = [
        for (final c in _allConstraints)
          if (c['panel'] == piece) c,
      ];
      final ids = {for (final c in _constraints) c['id']};
      _constraintGroups = [
        for (final g in (constraints?['groups'] as List?) ?? const [])
          if ((g['constraints'] as List).any(ids.contains)) g,
      ];
      final history = _run({'type': 'history.list'}) as Map<String, dynamic>?;
      _history = [
        for (final step in (history?['steps'] as List<dynamic>? ?? const []))
          step['label'] as String,
      ];
      _historyAt = (history?['at'] as int?) ?? 0;
    });
  }

  void _onFrame(bool board, double ms) {
    if (board) {
      _boardMs = _boardMs * 0.9 + ms * 0.1;
      return;
    }
    _garmentMs = _garmentMs * 0.9 + ms * 0.1;
    _frames++;
    final photo = _photo;
    if (photo != null && !photo.finished) {
      final now = _guard(widget.engine.photoStatus);
      if (now != null &&
          (now.samples != photo.samples ||
              now.finished ||
              (now.seconds - photo.seconds).abs() >= 1)) {
        setState(() => _photo = now);
      }
    }
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
      _toast(context.error(e));
      return null;
    }
  }

  /// Put the tape away: its band off the cloth, the readings cleared.
  void _clearTape() {
    _guard(widget.engine.clearTape);
    _tape = null;
    _lengthFrom = null;
    _length = null;
  }

  /// A click with the tape on pixel (`x`, `y`) of 3D view `view`: round
  /// the cloth there, or the first or second point of a length.
  void _measure(int view, double x, double y) {
    final engine = widget.engine;
    if (!_tapeLength) {
      final reading = _guard(() => engine.tapeAt(view: view, x: x, y: y));
      setState(() => _tape = reading);
      return;
    }
    final point = _guard(() => engine.clothPoint(view: view, x: x, y: y));
    if (point == null) return;
    final from = _lengthFrom;
    if (from == null) {
      _guard(engine.clearTape);
      setState(() {
        _lengthFrom = point;
        _length = null;
      });
      return;
    }
    final reading = _guard(() => engine.lengthBetween(from: from, to: point));
    setState(() {
      _lengthFrom = null;
      _length = reading;
    });
  }

  /// Start, pause or rebuild the drape, off the UI thread: placing the
  /// pieces around the body takes a moment, fitting a new body seconds.
  Future<void> _drape(
    Future<SimStatus> Function() call, {
    String? title,
  }) async {
    try {
      final status = await runBusy(
        context,
        widget.engine,
        title ?? context.l.busyDrapeTitle,
        call,
      );
      if (mounted) setState(() => _sim = status);
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
    }
  }

  /// Run or pause the drape; running it brings the 3D view back.
  void _setRunning(bool running) {
    if (running) {
      // An imported body with something to check says so; one that
      // cannot be draped does not start.
      final concern = checkConcern(_bodyCheck);
      if (concern != null) _bodyNotice();
      if (concern == 'error') return;
    }
    if (running && _maximized == ViewKind.board) {
      _resumeDrape = false;
      setState(() => _maximized = null);
    }
    _drape(() => widget.engine.runSimulation(running: running));
  }

  /// Make the avatar move (`walk`, `idle`, `turn`, `raise_arms`) or stand still
  /// (`null`); the drape runs, and the 3D view comes back if needed.
  void _setMotion(String? motion) {
    if (_maximized == ViewKind.board) {
      _resumeDrape = false;
      setState(() => _maximized = null);
    }
    _drape(() => widget.engine.setMotion(motion: motion));
  }

  /// Ask for a BVH motion capture, copy it into the project and play it.
  Future<void> _importMotion() async {
    final l = context.l;
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(label: l.motionFiles, extensions: const ['bvh']),
      ],
    );
    if (file == null || !mounted) return;
    final String motion;
    try {
      motion = await widget.engine.importMotion(path: file.path);
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
      return;
    }
    if (!mounted) return;
    _setMotion(motion);
    // Offered after the import rather than asked before it.
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(l.motionImported),
          behavior: SnackBarBehavior.floating,
          action: SnackBarAction(
            label: l.motionAddToLibrary,
            onPressed: () {
              try {
                final name = addLibraryMotion(
                  libraryDir: widget.libraryDir,
                  path: file.path,
                );
                if (mounted) _toast(context.l.motionAddedToLibrary(name));
              } on ApiError catch (e) {
                if (mounted) _toast(context.error(e));
              }
            },
          ),
        ),
      );
  }

  /// Play the motion `name` of the library: the project gets a copy.
  Future<void> _useLibraryMotion(String name) async {
    final String motion;
    try {
      motion = await widget.engine.useLibraryMotion(
        libraryDir: widget.libraryDir,
        name: name,
      );
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
      return;
    }
    if (mounted) _setMotion(motion);
  }

  void _showMotionLibrary() => showMotionLibrary(
    context,
    libraryDir: widget.libraryDir,
    onPlay: _useLibraryMotion,
  );

  /// The pattern library: a pattern added becomes a garment, draped from
  /// around the body.
  void _showPatternLibrary() => showPatternLibrary(
    context,
    engine: widget.engine,
    libraryDir: widget.libraryDir,
    add: (step) async {
      await _runLong(step);
      if (!mounted) return;
      _refresh();
      await _restart();
    },
  );

  /// Rebuild the simulation from the pattern, keeping it running if it was.
  Future<void> _restart() async {
    final wasRunning = _sim?.running ?? false;
    await _drape(widget.engine.resetSimulation);
    if (wasRunning && mounted) {
      await _drape(() => widget.engine.runSimulation(running: true));
    }
  }

  /// Ask how to render a photo, then show it refining in the 3D view.
  Future<void> _askPhoto() async {
    final choice = await askPhoto(context, _photoChoice);
    if (choice == null || !mounted) return;
    _photoChoice = choice;
    final status = _guard(
      () => widget.engine.startPhoto(request: choice.request),
    );
    if (status != null) setState(() => _photo = status);
  }

  void _stopPhoto() => _guard(widget.engine.stopPhoto);

  void _closePhoto() {
    _guard(widget.engine.closePhoto);
    setState(() => _photo = null);
  }

  /// Ask where to save the photo as it is now, and save it there (PNG).
  Future<void> _savePhoto() async {
    final l = context.l;
    final name = (_summary['name'] as String?) ?? 'Rustle3D';
    final location = await getSaveLocation(
      suggestedName: '$name.png',
      acceptedTypeGroups: [
        const XTypeGroup(label: 'PNG', extensions: ['png']),
      ],
    );
    if (location == null || !mounted) return;
    try {
      await widget.engine.savePhoto(path: location.path);
      if (!mounted) return;
      _toast(l.photoSaved(location.path.split(RegExp(r'[/\\]')).last));
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
    }
  }

  /// The Sew tool's first side, once picked (as `seam.add` takes it).
  Map<String, Object>? _sewFirst;

  /// Edges ⌘-clicked so far for the side being picked.
  final List<String> _sewChain = [];

  /// A press on a track with the Sew tool: where it started, where the
  /// pointer is along it now, and the board's scale then.
  (SewSpot, SewSpot, double)? _sewDrag;

  /// Where the Sew tool would pick, under the pointer.
  SewSpot? _sewHover;

  /// The Dimension tool: the corners or edges picked so far.
  final List<BoardPick> _dimPicks = [];

  /// The corner or edge under the pointer, and where the pointer is (mm).
  BoardPick? _dimHover;
  Offset? _dimPointer;

  /// A dimension just placed, and where its value is typed (board
  /// pixels).
  (Map<String, dynamic>, Offset)? _dimField;

  /// Where the pointer last went down on the board (board pixels).
  Offset _boardDown = Offset.zero;

  /// The Polygon tool's corners so far (mm).
  final List<Offset> _polygon = [];

  /// Corners picked on the board (one, or two to measure between).
  List<PickedPoint> _points = const [];

  /// The curve handle being dragged (Bend, Select): its edge, and `c1` or
  /// `c2`.
  (String, String)? _handle;

  /// The selected piece being turned or scaled by its frame's handles.
  _FrameDrag? _frameDrag;

  /// The points placed so far with Curve (a closed piece) or Cut (an open
  /// line), in mm.
  final List<_Anchor> _path = [];

  /// The tool of each rail group last used, shown on its button.
  final Map<String, String> _groupShown = {};

  /// Polygon and Curve draw half a piece, mirrored on the closing edge.
  bool _symmetric = false;

  /// Where the pointer is over the board while drawing, snapped.
  Snap? _hover;

  /// The construction line's first point, once clicked.
  Offset? _guideStart;

  /// Colour the cloth by a fit map (`strain`, `gap`) or by its fabric
  /// (`null`); the legend follows the drape once a second.
  void _setFitMap(String? map) {
    final set = _guard(() {
      widget.engine.setFitMap(map: map);
      return true;
    });
    if (set == null) return;
    _fitPoll?.cancel();
    _fitPoll = map == null
        ? null
        : Timer.periodic(const Duration(seconds: 1), (_) {
            if (mounted) setState(() => _fitLegend = _legend());
          });
    setState(() {
      _fitMap = map;
      _fitLegend = map == null ? null : _legend();
    });
  }

  /// The fit map's legend now, `null` without one or on an error.
  FitLegend? _legend() {
    try {
      return widget.engine.fitLegend();
    } on ApiError {
      return null;
    }
  }

  void _setTool(String tool) => setState(() {
    if (_tool == 'tape' && tool != 'tape') _clearTape();
    _tool = tool;
    for (final (group, tools) in _ToolRail.groups) {
      if (tools.contains(tool)) _groupShown[group] = tool;
    }
    _sewFirst = null;
    _sewChain.clear();
    _sewDrag = null;
    _sewHover = null;
    _dimPicks.clear();
    _dimHover = null;
    _dimPointer = null;
    _polygon.clear();
    _path.clear();
    _guideStart = null;
    _hover = null;
    _buttonFirst = null;
    _zipFirst = null;
  });

  /// The points a new point may line up with: the first and the last
  /// placed so far.
  List<double> get _alignWith {
    final placed = [..._polygon, for (final a in _path) a.at, ?_guideStart];
    return [
      if (placed.isNotEmpty) ...[placed.last.dx, placed.last.dy],
      if (placed.length > 2) ...[placed.first.dx, placed.first.dy],
    ];
  }

  /// Where a point placed at `mm` lands: on a corner, an edge's middle,
  /// an edge, a construction line or an axis, or level with the points
  /// placed so far.
  Offset _snapped(Offset mm, double mmPerPx) {
    final s = _guard(
      () => widget.engine.snap(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
        align: _alignWith,
      ),
    );
    return s == null ? mm : Offset(s.xMm, s.yMm);
  }

  /// The pointer moved over the board while drawing: show where a click
  /// would land.
  void _hoverAt(Offset mm, double mmPerPx) {
    final s = _guard(
      () => widget.engine.snap(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
        align: _alignWith,
      ),
    );
    setState(() => _hover = s);
  }

  /// Construction line: the first click starts it, the second ends it.
  void _guideTap(Offset mm, double mmPerPx) {
    final at = _snapped(mm, mmPerPx);
    final start = _guideStart;
    if (start == null) {
      setState(() => _guideStart = at);
      return;
    }
    if ((at - start).distance < 1e-6) return;
    final out = _run({
      'type': 'guide.add',
      'a': [start.dx, start.dy],
      'b': [at.dx, at.dy],
    });
    setState(() => _guideStart = null);
    _refresh();
    if (out != null) _toast(context.l.toastGuideAdded);
  }

  /// Hole: the corners clicked cut a hole in the selected piece.
  void _finishHole() {
    final l = context.l;
    final pts = List.of(_polygon);
    setState(_polygon.clear);
    if (_selection.length != 1) return _toast(l.holeSelectFirst);
    final [first, ...rest] = pts;
    final out = _run({
      'type': 'panel.add_hole',
      'panel': _selection.first,
      'start': [first.dx, first.dy],
      'segments': [
        for (final p in rest)
          {
            'kind': 'line',
            'to': [p.dx, p.dy],
          },
      ],
    });
    _refresh();
    if (out != null) _toast(l.toastHoleAdded);
  }

  /// Where on an edge a click landed, or null off the edges.
  EdgeSpot? _spot(Offset mm, double mmPerPx) => _guard<EdgeSpot?>(
    () => widget.engine.edgeSpotNear(
      xMm: mm.dx,
      yMm: mm.dy,
      maxMm: _pickPx * mmPerPx,
    ),
  );

  /// Button: a click where the button goes, then one where it buttons
  /// (the buttonhole's end, on the piece lapped over it).
  void _buttonTap(Offset mm, double mmPerPx) {
    final l = context.l;
    final panel = _guard(
      () => widget.engine.pickPieceAt(xMm: mm.dx, yMm: mm.dy),
    );
    if (panel == null) return _toast(l.buttonClickPiece);
    final first = _buttonFirst;
    if (first == null) {
      _buttonFirst = (panel, mm);
      return _refresh();
    }
    _buttonFirst = null;
    final out = _run({
      'type': 'button.add',
      'panel': first.$1,
      'at': [first.$2.dx, first.$2.dy],
      'hole_panel': panel,
      'hole_at': [mm.dx, mm.dy],
    });
    _refresh();
    if (out != null) _toast(l.toastButtonAdded);
  }

  /// Zipper: a click on one edge near the zipper's bottom, then on the
  /// edge it zips to; the zipper runs along both edges.
  void _zipperTap(Offset mm, double mmPerPx) {
    final spot = _spot(mm, mmPerPx);
    if (spot == null) return;
    final fromEnd = spot.offsetMm > spot.lengthMm / 2;
    final first = _zipFirst;
    if (first == null || first.$1 == spot.edge) {
      setState(() => _zipFirst = (spot.edge, fromEnd));
      return;
    }
    setState(() => _zipFirst = null);
    final out = _run({
      'type': 'zipper.add',
      'a': {'edge': first.$1, 'reversed': first.$2},
      'b': {'edge': spot.edge},
    });
    _refresh();
    if (out != null) _toast(context.l.toastZipperAdded);
  }

  /// Dart: drawn, folded and sewn where the edge was clicked.
  Future<void> _dartTap(Offset mm, double mmPerPx) async {
    final spot = _spot(mm, mmPerPx);
    if (spot == null) return;
    final l = context.l;
    final picked = await _askNumbers(
      context,
      title: l.toolDart,
      fields: [(l.dartWidth, l.cm, 2), (l.dartLength, l.cm, 10)],
    );
    if (picked == null || !mounted) return;
    final [w, len] = picked.$1;
    _run({
      'type': 'dart.add',
      'edge': spot.edge,
      'at_mm': spot.offsetMm,
      'width_mm': w * 10,
      'length_mm': len * 10,
    });
    _refresh();
  }

  /// Pleat: folded and sewn from where the edge was clicked.
  Future<void> _pleatTap(Offset mm, double mmPerPx) async {
    final spot = _spot(mm, mmPerPx);
    if (spot == null) return;
    final l = context.l;
    final picked = await _askNumbers(
      context,
      title: l.toolPleat,
      fields: [(l.pleatDepth, l.cm, 3), (l.pleatStitched, l.cm, 0)],
      choices: [l.pleatKnife, l.pleatBox],
    );
    if (picked == null || !mounted) return;
    final ([depth, stitched], choice) = picked;
    _run({
      'type': 'pleat.add',
      'edge': spot.edge,
      'at_mm': spot.offsetMm,
      'depth_mm': depth * 10,
      'kind': choice == 0 ? 'knife' : 'box',
      if (stitched > 0) 'stitched_mm': stitched * 10,
    });
    _refresh();
  }

  /// Arc: an edge clicked becomes a circular arc, bulging as typed.
  Future<void> _arcTap(Offset mm, double mmPerPx) async {
    _guard(
      () => widget.engine.pickEdgeNear(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
      ),
    );
    _refresh();
    final e = _pickedEdge;
    if (e == null) return;
    await _askArc(e['id'] as String, (e['length_mm'] as num).toDouble());
  }

  Future<void> _askArc(String edge, double lengthMm) async {
    final l = context.l;
    final picked = await _askNumbers(
      context,
      title: l.arcTitle,
      fields: [(l.arcBulge, l.cm, (lengthMm / 80).clamp(0.5, 20))],
      hint: l.arcHint,
    );
    if (picked == null || !mounted) return;
    final bulge = picked.$1.first;
    if (bulge == 0) return;
    _run({'type': 'edge.set_arc', 'edge': edge, 'bulge_mm': bulge * 10});
    _refresh();
  }

  /// Right of the pieces on the board, at the height of the first one.
  List<double> _freeSpot(double margin) {
    final panels = (_summary['panels'] as List<dynamic>? ?? const []);
    var right = 0.0;
    for (final p in panels) {
      final max = (p['bbox']?['max'] as List<dynamic>?)?.first as num?;
      if (max != null && max > right) right = max.toDouble();
    }
    return [right + 60 + margin, 0];
  }

  Future<void> _addEllipse() async {
    final l = context.l;
    final picked = await _askNumbers(
      context,
      title: l.toolEllipse,
      fields: [(l.width, l.cm, 30), (l.height, l.cm, 20)],
    );
    if (picked == null || !mounted) return;
    final [w, h] = picked.$1;
    if (w <= 0 || h <= 0) return;
    final spot = _freeSpot(w * 5);
    final panels = (_summary['panels'] as List<dynamic>? ?? const []);
    _shapeAdded(
      _run({
        'type': 'panel.create_ellipse',
        'name': l.pieceNew(panels.length + 1),
        'centre': [spot[0], h * 5],
        'radii': [w * 5, h * 5],
      }),
    );
  }

  /// After a shape is added: back to Select with the new pieces selected,
  /// so the next click picks a piece rather than going to the tool used
  /// before. `out` is a command's output or a batch's list of them.
  void _shapeAdded(dynamic out) {
    if (out == null) return;
    final panels = [
      for (final o in out is List ? out : [out])
        for (final id in ((o as Map?)?['created'] as List? ?? const []))
          if (id is String && id.startsWith('panel:')) id,
    ];
    _setTool('select');
    _guard(() => widget.engine.select(panels: panels));
    _refresh();
  }

  /// A circle skirt: the pieces of a full, half or quarter circle for the
  /// waist and length typed (a full circle is two half pieces).
  Future<void> _addSector() async {
    final l = context.l;
    final waist =
        ((_summary['avatar'] as Map?)?['body']?['waist_cm'] as num?)
            ?.toDouble() ??
        70;
    final picked = await _askNumbers(
      context,
      title: l.toolSector,
      fields: [(l.sectorWaist, l.cm, waist + 2), (l.sectorLength, l.cm, 60)],
      choices: [l.sectorFull, l.sectorHalf, l.sectorQuarter],
      hint: l.sectorHint,
    );
    if (picked == null || !mounted) return;
    final ([w, len], choice) = picked;
    if (w <= 0 || len <= 0) return;
    final (pieces, sweep) = switch (choice) {
      0 => (2, 180.0),
      1 => (1, 180.0),
      _ => (1, 90.0),
    };
    final panels = (_summary['panels'] as List<dynamic>? ?? const []);
    final inner = w * 10 / pieces;
    final radius = inner / (sweep * 3.141592653589793 / 180) + len * 10;
    final spot = _freeSpot(radius);
    final out = _run({
      'type': 'batch',
      'label': 'panel.create_sector',
      'commands': [
        for (var k = 0; k < pieces; k++)
          {
            'type': 'panel.create_sector',
            'name': l.pieceNew(panels.length + 1 + k),
            'centre': [spot[0] + k * (2 * radius + 60), 0],
            'inner_length_mm': inner,
            'length_mm': len * 10,
            'sweep_deg': sweep,
          },
      ],
    });
    _shapeAdded(out);
  }

  /// Board pick tolerance (logical pixels).
  static const _pickPx = 12.0;

  void _polygonTap(Offset mm, double mmPerPx) {
    if (_polygon.length >= 3 &&
        (mm - _polygon.first).distance < _pickPx * mmPerPx) {
      _finishPolygon();
      return;
    }
    final at = _snapped(mm, mmPerPx);
    setState(() => _polygon.add(at));
  }

  void _finishPolygon() {
    if (_polygon.length < 3) return;
    if (_tool == 'hole') return _finishHole();
    final panels = (_summary['panels'] as List<dynamic>? ?? const []);
    final out = _run({
      'type': 'panel.create_polygon',
      'name': context.l.pieceNew(panels.length + 1),
      'points': [
        for (final p in _polygon) [p.dx, p.dy],
      ],
      if (_symmetric) 'symmetric': true,
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

  /// Curve and Cut: a press places a point; dragging pulls its handle out
  /// (a smooth point, the curve leaving it towards the pointer). With
  /// Curve, a press on the first point closes the piece.
  bool _pathPress(Offset mm, double mmPerPx) {
    if (_tool == 'curve' &&
        _path.length >= 2 &&
        (mm - _path.first.at).distance < _pickPx * mmPerPx) {
      _finishPath();
      return true;
    }
    final at = _snapped(mm, mmPerPx);
    setState(() => _path.add(_Anchor(at)));
    return true;
  }

  void _pathDrag(Offset mm) {
    if (_path.isEmpty) return;
    setState(() => _path.last = _Anchor(_path.last.at, handle: mm));
  }

  /// The path's segments (`panel.create_path`, `panel.cut`); `closed`
  /// adds the edge back to the first point.
  static List<Map<String, Object>> _segments(
    List<_Anchor> pts, {
    required bool closed,
  }) {
    List<double> xy(Offset o) => [o.dx, o.dy];
    final n = pts.length;
    return [
      for (var i = 0; i < (closed ? n : n - 1); i++)
        if ((pts[i].handle, pts[(i + 1) % n].handleIn) case (final c1, final c2)
            when c1 != null || c2 != null)
          {
            'kind': 'cubic',
            'c1': xy(c1 ?? pts[i].at),
            'c2': xy(c2 ?? pts[(i + 1) % n].at),
            'to': xy(pts[(i + 1) % n].at),
          }
        else
          {'kind': 'line', 'to': xy(pts[(i + 1) % n].at)},
    ];
  }

  /// Enter: close the polygon or the curved piece, or make the cut.
  void _finish() {
    if (_tool == 'polygon' || _tool == 'hole') return _finishPolygon();
    if (_tool == 'sew' && _sewChain.isNotEmpty) {
      return _sewRun(List.of(_sewChain));
    }
    if (_tool == 'curve' || _tool == 'cut') return _finishPath();
  }

  void _finishPath() {
    if (_path.length < 2) return;
    final l = context.l;
    final start = [_path.first.at.dx, _path.first.at.dy];
    final Map<String, Object> step;
    if (_tool == 'curve') {
      final panels = (_summary['panels'] as List<dynamic>? ?? const []);
      step = {
        'type': 'panel.create_path',
        'name': l.pieceNew(panels.length + 1),
        'start': start,
        'segments': _segments(_path, closed: true),
        if (_symmetric) 'symmetric': true,
      };
    } else {
      step = {
        'type': 'panel.cut',
        'start': start,
        'segments': _segments(_path, closed: false),
      };
    }
    final out = _run(step);
    setState(_path.clear);
    _refresh();
    if (out != null) {
      _toast(_tool == 'curve' ? l.toastPieceAdded : l.toastCut);
    }
  }

  /// Straighten: a curved edge clicked becomes a straight line.
  void _straightenTap(Offset mm, double mmPerPx) {
    _guard(
      () => widget.engine.pickEdgeNear(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
      ),
    );
    _refresh();
    final e = _pickedEdge;
    if (e == null || e['curve']['kind'] != 'cubic') return;
    _run({
      'type': 'edge.set_curve',
      'edge': e['id'],
      'curve': {'kind': 'line'},
    });
    _refresh();
  }

  void _addPointTap(Offset mm, double mmPerPx) {
    final point = _guard(
      () => widget.engine.addPointNear(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
      ),
    );
    if (point != null) {
      _refresh();
      _toast(context.l.toastPointAdded);
    }
  }

  /// Bend: pick an edge; a straight one gets handles on its line.
  void _bendTap(Offset mm, double mmPerPx) {
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

  /// The curve handles shown on the board: both of the curved edge picked
  /// (Bend, Select), or, with Select, a picked corner's: one on each
  /// curve meeting there.
  List<(Map<String, dynamic>, String)> get _handles {
    if (_tool != 'bend' && _tool != 'select') return const [];
    bool cubic(Map<String, dynamic> e) => e['curve']['kind'] == 'cubic';
    if (_pickedEdge case final e? when cubic(e)) return [(e, 'c1'), (e, 'c2')];
    if (_tool != 'select' || _points.length != 1) return const [];
    final corner = _points.first.id;
    return [
      for (final e
          in (_panel?['edges'] as List? ?? const [])
              .cast<Map<String, dynamic>>())
        if (cubic(e)) ...[
          if (e['start']['id'] == corner) (e, 'c1'),
          if (e['end']['id'] == corner) (e, 'c2'),
        ],
    ];
  }

  /// The selected piece's bounding box (mm), with Select and nothing
  /// picked on it, when it can be turned and scaled by its frame (not a
  /// linked mirror copy: that one follows its source).
  Rect? get _frame {
    final panel = _panel;
    if (_tool != 'select' ||
        panel == null ||
        panel['mirror_of'] != null ||
        _pickedEdge != null ||
        _points.isNotEmpty) {
      return null;
    }
    final box = panel['bbox'] as Map<String, dynamic>?;
    if (box == null) return null;
    return Rect.fromPoints(_at(box['min']), _at(box['max']));
  }

  /// The frame's handles (mm) on a board at `mmPerPx`: each corner with
  /// the piece's bounding-box corner it stands for (set off from it, so
  /// the piece's own corners stay free to drag), then the turning handle
  /// above the piece.
  (List<(Offset, Offset)>, Offset)? _frameHandles(double mmPerPx) {
    final f = _frame;
    if (f == null) return null;
    // Board y goes up: the rectangle's bottom is the piece's top.
    final r = f.inflate(12 * mmPerPx);
    return (
      [
        (r.topLeft, f.topLeft),
        (r.topRight, f.topRight),
        (r.bottomRight, f.bottomRight),
        (r.bottomLeft, f.bottomLeft),
      ],
      Offset(f.center.dx, r.bottom + 28 * mmPerPx),
    );
  }

  /// A press on a handle of the selected piece's frame: the turning
  /// handle turns it about its centre, a corner scales it from the
  /// opposite corner.
  bool _framePress(Offset mm, double mmPerPx) {
    final id = _panel?['id'] as String?;
    final f = _frame;
    final handles = _frameHandles(mmPerPx);
    if (id == null || f == null || handles == null) return false;
    final (corners, turn) = handles;
    final near = _pickPx * mmPerPx;
    if ((turn - mm).distance < near) {
      _frameDrag = _FrameDrag(id, f.center, mm, null);
      return true;
    }
    for (final (i, (handle, corner)) in corners.indexed) {
      if ((handle - mm).distance < near) {
        _frameDrag = _FrameDrag(id, corners[(i + 2) % 4].$2, mm, corner);
        return true;
      }
    }
    return false;
  }

  /// Turn (in whole degrees, 15° steps with ⇧) or scale (both ways at
  /// once with ⇧) the piece to follow the pointer, by what is left to
  /// apply since the last move.
  void _frameDragTo(Offset mm) {
    final d = _frameDrag;
    if (d == null || d.refused) return;
    final shift = HardwareKeyboard.instance.isShiftPressed;
    final Map<String, Object?> step;
    var angle = d.angle;
    var factor = d.factor;
    if (d.corner case final corner?) {
      final from = corner - d.pivot;
      final now = corner + (mm - d.grab) - d.pivot;
      double ratio(double to, double was) =>
          was.abs() < 1e-9 ? 1 : (to / was).clamp(0.05, 20);
      if (shift && from.distanceSquared > 1e-9) {
        final k = ((now.dx * from.dx + now.dy * from.dy) / from.distanceSquared)
            .clamp(0.05, 20.0);
        factor = Offset(k, k);
      } else {
        factor = Offset(ratio(now.dx, from.dx), ratio(now.dy, from.dy));
      }
      if (factor == d.factor) return;
      step = {
        'type': 'panel.scale',
        'panel': d.panel,
        'factor': [factor.dx / d.factor.dx, factor.dy / d.factor.dy],
        'centre': [d.pivot.dx, d.pivot.dy],
      };
    } else {
      double direction(Offset v) => atan2(v.dy, v.dx) * 180 / pi;
      final swept =
          (direction(mm - d.pivot) - direction(d.grab - d.pivot) + 180) % 360 -
          180;
      angle = shift ? (swept / 15).round() * 15.0 : swept.roundToDouble();
      if (angle == d.angle) return;
      step = {
        'type': 'panel.rotate',
        'panel': d.panel,
        'angle_deg': angle - d.angle,
        'centre': [d.pivot.dx, d.pivot.dy],
      };
    }
    try {
      widget.engine.run(stepJson: jsonEncode(step));
      d
        ..angle = angle
        ..factor = factor;
    } on ApiError catch (e) {
      // Refused once (a constraint): not again for every move.
      d.refused = true;
      _toast(context.error(e));
    }
    _refresh();
  }

  void _frameRelease() {
    if (_frameDrag == null) return;
    _frameDrag = null;
    // The whole drag is one undo step.
    _guard(widget.engine.release);
  }

  /// A press on a curve handle holds it; elsewhere it does what the tool
  /// does.
  bool _handlePress(Offset mm, double mmPerPx) {
    for (final (e, key) in _handles) {
      if ((_at(e['curve'][key]) - mm).distance < _pickPx * mmPerPx) {
        _handle = (e['id'] as String, key);
        return true;
      }
    }
    return false;
  }

  void _handleDrag(Offset mm) {
    final (id, key) = _handle ?? ('', '');
    final e = (_panel?['edges'] as List? ?? const [])
        .cast<Map<String, dynamic>>()
        .where((e) => e['id'] == id)
        .firstOrNull;
    if (e == null) return;
    final c1 = key == 'c1' ? mm : _at(e['curve']['c1']);
    final c2 = key == 'c2' ? mm : _at(e['curve']['c2']);
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

  void _handleRelease() {
    if (_handle == null) return;
    _handle = null;
    // The whole drag is one undo step.
    _guard(widget.engine.release);
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

    if (_tool == 'sew') _paintSew(canvas, toScreen, p);
    if (_tool == 'dimension') _paintDimension(canvas, toScreen, p);
    if (_buttonFirst case (_, final at) when _tool == 'button') {
      canvas.drawCircle(toScreen(at), 7, line);
      point(toScreen(at), 2.5);
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
    if (_path.isNotEmpty) {
      final cut = _tool == 'cut';
      final pen = Paint()
        ..color = cut ? p.danger : p.accent
        ..strokeWidth = 1.5
        ..style = PaintingStyle.stroke;
      final path = Path();
      final first = toScreen(_path.first.at);
      path.moveTo(first.dx, first.dy);
      for (final s in _segments(_path, closed: false)) {
        Offset at(String key) {
          final v = s[key]! as List<double>;
          return toScreen(Offset(v[0], v[1]));
        }

        final to = at('to');
        if (s['kind'] == 'cubic') {
          final (c1, c2) = (at('c1'), at('c2'));
          path.cubicTo(c1.dx, c1.dy, c2.dx, c2.dy, to.dx, to.dy);
        } else {
          path.lineTo(to.dx, to.dy);
        }
      }
      canvas.drawPath(path, pen);
      for (final (i, a) in _path.indexed) {
        if (a.handle case final h?) {
          final (o, hi) = (toScreen(h), toScreen(a.handleIn!));
          canvas.drawLine(hi, o, line);
          point(o, 3);
          point(hi, 3);
        }
        point(toScreen(a.at), i == 0 && !cut ? 6 : 4);
      }
    }
    // The selected piece's frame: corners to scale it, a handle on a
    // stalk to turn it.
    final pxPerMm =
        (toScreen(const Offset(1, 0)) - toScreen(Offset.zero)).distance;
    if (_frameHandles(pxPerMm > 0 ? 1 / pxPerMm : 1) case (
      final corners,
      final turn,
    )) {
      final frame = Paint()
        ..color = p.accent.withValues(alpha: 0.6)
        ..strokeWidth = 1
        ..style = PaintingStyle.stroke;
      final screen = [for (final (h, _) in corners) toScreen(h)];
      canvas.drawPath(Path()..addPolygon(screen, true), frame);
      final top = toScreen(Offset(turn.dx, corners[2].$1.dy));
      canvas.drawLine(top, toScreen(turn), frame);
      for (final c in screen) {
        canvas.drawRect(Rect.fromCenter(center: c, width: 9, height: 9), ring);
        canvas.drawRect(Rect.fromCenter(center: c, width: 7, height: 7), dot);
      }
      point(toScreen(turn), 5);
    }
    for (final (e, key) in _handles) {
      final end = e[key == 'c1' ? 'start' : 'end']['pos'];
      final h = toScreen(_at(e['curve'][key]));
      canvas.drawLine(toScreen(_at(end)), h, line);
      point(h, 5);
    }
    // The line or edge being drawn follows the pointer.
    if (_hover case final h?) {
      final placed = [..._polygon, for (final a in _path) a.at, ?_guideStart];
      if (placed.isNotEmpty) {
        canvas.drawLine(
          toScreen(placed.last),
          toScreen(Offset(h.xMm, h.yMm)),
          line..color = p.accent.withValues(alpha: 0.5),
        );
        line.color = p.accent;
      }
      _paintSnap(context, canvas, toScreen, h, p);
    }
    if (_guideStart case final g?) point(toScreen(g), 4);
    if (_tool == 'select') {
      if (_guard(widget.engine.dragSnap) case final s?) {
        _paintSnap(context, canvas, toScreen, s, p);
      }
    }
    for (final g in _guideList.cast<Map<String, dynamic>>()) {
      if (!_guides.contains(g['id'])) continue;
      final (a, b) = (_at(g['a']), _at(g['b']));
      final d = (b - a) / max((b - a).distance, 1e-9);
      canvas.drawLine(
        toScreen(a - d * 1e5),
        toScreen(b + d * 1e5),
        Paint()
          ..color = p.accent.withValues(alpha: 0.45)
          ..strokeWidth = 4,
      );
    }
    for (final n in _notches) {
      final at = toScreen(Offset(n.xMm, n.yMm));
      canvas.drawCircle(at, 7, line);
      point(at, 3);
    }
    _labelRects
      ..clear()
      ..addAll(
        _paintConstraints(
          context,
          canvas,
          toScreen,
          _allConstraints,
          _selection,
          p,
        ),
      );
    // A dimension just added: its field opens on its label.
    if (_fieldFor case final id?) {
      final placed = _labelRects.where((r) => r.$1['id'] == id).firstOrNull;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        setState(() {
          _fieldFor = null;
          if (placed != null) {
            _dimField = (placed.$1, placed.$2.center - const Offset(0, 22));
          }
        });
      });
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

  /// Open the value field of dimension `id` where its label is drawn
  /// (once drawn).
  void _openField(String id) => setState(() => _fieldFor = id);

  /// A press on a dimension's label (Select): held, to drag the label or,
  /// without moving, to open its value field.
  bool _labelPress(Offset mm) {
    final hit = _labelRects.reversed
        .where((r) => r.$2.inflate(2).contains(_boardDown))
        .firstOrNull;
    if (hit == null) return false;
    final c = hit.$1;
    final at = c['label'] == null ? Offset.zero : _at(c['label']);
    _labelDrag = (c, at, mm, false);
    return true;
  }

  void _labelDragTo(Offset mm) {
    final (c, at, from, _) = _labelDrag!;
    _labelDrag = (c, at, from, true);
    final to = at + (mm - from);
    // One undo step for the whole drag (the command merges).
    _run({
      'type': 'constraint.place_label',
      'constraint': c['id'],
      'offset': [to.dx, to.dy],
    });
    _refresh();
  }

  void _labelRelease() {
    final drag = _labelDrag;
    if (drag == null) return;
    _labelDrag = null;
    if (drag.$4) return;
    // A click on a label: its value field.
    final rect = _labelRects.where((r) => r.$1['id'] == drag.$1['id']);
    if (rect.isEmpty) return;
    setState(
      () => _dimField = (drag.$1, rect.first.$2.center - const Offset(0, 22)),
    );
  }

  /// The pick's points on the board (mm): a corner, or an edge's polyline.
  static List<Offset> _pickPoints(BoardPick pick) => [
    for (var i = 0; i + 1 < pick.points.length; i += 2)
      Offset(pick.points[i], pick.points[i + 1]),
  ];

  static bool _isCorner(BoardPick pick) => pick.id.startsWith('point:');

  void _dimHoverAt(Offset mm, double mmPerPx) => setState(() {
    _dimPointer = mm;
    _dimHover = _guard<BoardPick?>(
      () => widget.engine.cornerOrEdgeNear(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
      ),
    );
  });

  /// A distance between two corners, pulled out to `mm`: a width when
  /// pulled above or below both, a height when pulled beside both,
  /// straight between them otherwise.
  String _dimAlong(Offset a, Offset b, Offset mm) {
    final inX = mm.dx > min(a.dx, b.dx) && mm.dx < max(a.dx, b.dx);
    final inY = mm.dy > min(a.dy, b.dy) && mm.dy < max(a.dy, b.dy);
    if (inX && !inY) return 'horizontal';
    if (inY && !inX) return 'vertical';
    return 'direct';
  }

  /// Dimension: click an edge (its length), two corners (the distance
  /// between them) or two edges (the angle between them), then click where
  /// the dimension goes; its value is typed there.
  void _dimTap(Offset mm, double mmPerPx) {
    final pick = _guard<BoardPick?>(
      () => widget.engine.cornerOrEdgeNear(
        xMm: mm.dx,
        yMm: mm.dy,
        maxMm: _pickPx * mmPerPx,
      ),
    );
    final picks = _dimPicks;
    if (picks.isEmpty) {
      if (pick != null) setState(() => picks.add(pick));
      return;
    }
    final first = picks.first;
    final another = pick != null && pick.id != first.id;
    if (picks.length == 1 && _isCorner(first)) {
      // A corner is measured to a second corner.
      if (another && _isCorner(pick)) setState(() => picks.add(pick));
      return;
    }
    if (picks.length == 1 && another && !_isCorner(pick)) {
      return setState(() => picks.add(pick));
    }
    final c = switch (picks) {
      [final a, final b] when _isCorner(a) => {
        'kind': 'distance',
        'a': a.id,
        'b': b.id,
        'mm': 1,
        'along': _dimAlong(_pickPoints(a).first, _pickPoints(b).first, mm),
      },
      [final a, final b] => {'kind': 'angle', 'a': a.id, 'b': b.id, 'deg': 90},
      _ => {'kind': 'length', 'edge': first.id, 'mm': 1},
    };
    // The label stays where the dimension was placed.
    Offset halfway(List<Offset> pts) {
      final total = [
        for (var i = 1; i < pts.length; i++) (pts[i] - pts[i - 1]).distance,
      ].fold(0.0, (a, b) => a + b);
      var walked = 0.0;
      for (var i = 1; i < pts.length; i++) {
        final len = (pts[i] - pts[i - 1]).distance;
        if (walked + len >= total / 2 && len > 0) {
          return Offset.lerp(pts[i - 1], pts[i], (total / 2 - walked) / len)!;
        }
        walked += len;
      }
      return pts.first;
    }

    final marks = [
      for (final pick in picks)
        _isCorner(pick) ? _pickPoints(pick).first : halfway(_pickPoints(pick)),
    ];
    if (_labelAnchor(c, marks) case final anchor?) {
      final off = mm - anchor;
      c['label'] = [off.dx, off.dy];
    }
    setState(picks.clear);
    final out = _run({
      'type': 'constraint.add',
      'constraint': c,
      'measure': true,
    });
    if (out == null) return;
    final id = (out['created'] as List?)?.firstOrNull;
    _guard(() => widget.engine.select(panels: [first.panel]));
    _refresh();
    final placed = _constraints.cast<Map<String, dynamic>>().where(
      (c) => c['id'] == id,
    );
    if (placed.isNotEmpty) {
      setState(() => _dimField = (placed.first, _boardDown));
    }
  }

  /// The Dimension tool: what is under the pointer, what is picked, and
  /// the dimension being pulled out with its value.
  void _paintDimension(
    Canvas canvas,
    Offset Function(Offset mm) toScreen,
    Palette p,
  ) {
    Paint pen(double alpha, double width) => Paint()
      ..color = p.accent.withValues(alpha: alpha)
      ..strokeWidth = width
      ..strokeCap = StrokeCap.round
      ..style = PaintingStyle.stroke;
    void show(BoardPick pick, Paint paint) {
      final pts = [for (final q in _pickPoints(pick)) toScreen(q)];
      if (pts.length == 1) {
        canvas.drawCircle(pts.first, 6, paint);
        return;
      }
      for (var i = 1; i < pts.length; i++) {
        canvas.drawLine(pts[i - 1], pts[i], paint);
      }
    }

    if (_dimHover case final h?) show(h, pen(0.35, 5));
    for (final pick in _dimPicks) {
      show(pick, pen(1, 3));
    }
    final at = _dimPointer;
    if (at == null) return;
    final cm = context.l.cm;
    switch (_dimPicks) {
      case [final a, final b] when _isCorner(a):
        final (pa, pb) = (_pickPoints(a).first, _pickPoints(b).first);
        final along = _dimAlong(pa, pb, at);
        final (s, e, mm) = switch (along) {
          'horizontal' => (
            Offset(pa.dx, at.dy),
            Offset(pb.dx, at.dy),
            (pb.dx - pa.dx).abs(),
          ),
          'vertical' => (
            Offset(at.dx, pa.dy),
            Offset(at.dx, pb.dy),
            (pb.dy - pa.dy).abs(),
          ),
          _ => (pa, pb, (pb - pa).distance),
        };
        final dash = pen(0.7, 1);
        canvas.drawLine(toScreen(pa), toScreen(s), dash);
        canvas.drawLine(toScreen(pb), toScreen(e), dash);
        canvas.drawLine(toScreen(s), toScreen(e), pen(1, 1.5));
        _pill(
          canvas,
          Offset.lerp(toScreen(s), toScreen(e), 0.5)!,
          '${context.number(mm / 10, 1)} $cm',
          p.surface,
          p.accent,
        );
      case [final a] when !_isCorner(a):
        final pts = _pickPoints(a);
        var mm = 0.0;
        for (var i = 1; i < pts.length; i++) {
          mm += (pts[i] - pts[i - 1]).distance;
        }
        _pill(
          canvas,
          toScreen(at) + const Offset(0, -18),
          '${context.number(mm / 10, 1)} $cm',
          p.surface,
          p.accent,
        );
    }
  }

  /// Where the Sew tool picks at `mm`: on the edge or line there, or
  /// along track `on`; snapped to ends, notches and other seams' ends.
  SewSpot? _sewSpot(Offset mm, double mmPerPx, {String? on}) =>
      _guard<SewSpot?>(
        () => widget.engine.sewSpotNear(
          xMm: mm.dx,
          yMm: mm.dy,
          maxMm: _pickPx * mmPerPx,
          along: on,
        ),
      );

  void _sewHoverAt(Offset mm, double mmPerPx) =>
      setState(() => _sewHover = _sewSpot(mm, mmPerPx));

  /// Sew tool: a press on an edge or a line holds it (a click picks it
  /// whole, a drag picks the stretch dragged along); elsewhere it pans.
  bool _sewPress(Offset mm, double mmPerPx) {
    final spot = _sewSpot(mm, mmPerPx);
    if (spot == null) return false;
    setState(() => _sewDrag = (spot, spot, mmPerPx));
    return true;
  }

  void _sewDragTo(Offset mm) {
    final drag = _sewDrag;
    if (drag == null) return;
    final (start, _, mmPerPx) = drag;
    final spot = _sewSpot(mm, mmPerPx, on: start.track);
    if (spot != null) setState(() => _sewDrag = (start, spot, mmPerPx));
  }

  void _sewRelease() {
    final drag = _sewDrag;
    if (drag == null) return;
    setState(() => _sewDrag = null);
    final (start, end, mmPerPx) = drag;
    final key = start.track.startsWith('line:') ? 'line' : 'edge';
    final (from, to) = start.offsetMm < end.offsetMm
        ? (start.offsetMm, end.offsetMm)
        : (end.offsetMm, start.offsetMm);
    // Dragged along: that stretch only (all of it when dragged end to end).
    if (to - from > _pickPx * mmPerPx) {
      final whole = from < 1e-6 && to > start.lengthMm - 1e-6;
      return _sewSide({
        key: start.track,
        if (!whole) 'from_mm': from,
        if (!whole) 'to_mm': to,
      });
    }
    if (key == 'line') return _sewSide({'line': start.track});
    final keys = HardwareKeyboard.instance;
    if (keys.isControlPressed || keys.isMetaPressed || keys.isShiftPressed) {
      // ⌘-click: one more edge of a run sewn as one side (again: one less).
      return setState(() {
        if (!_sewChain.remove(start.track)) _sewChain.add(start.track);
      });
    }
    _sewRun([..._sewChain.where((e) => e != start.track), start.track]);
  }

  /// The edges ⌘-clicked, as one side, in their order around the piece.
  void _sewRun(List<String> edges) {
    final run = _guard(() => widget.engine.sewChain(edges: edges)) ?? edges;
    _sewSide(run.length == 1 ? {'edge': run.first} : {'edges': run});
  }

  /// A side picked: the first is kept, the second sews them.
  void _sewSide(Map<String, Object> side) {
    final first = _sewFirst;
    setState(() {
      _sewChain.clear();
      _sewFirst = first == null ? side : null;
    });
    if (first == null) return;
    final out = _run({'type': 'seam.add', 'a': first, 'b': side});
    _refresh();
    if (out != null) _toast(context.l.toastSewn);
  }

  /// Where a seam side runs on the board (mm).
  List<Offset> _sidePoints(Map<String, Object> side) {
    // As the document stores it: one edge is a run of one.
    final stored = {
      for (final MapEntry(:key, :value) in side.entries)
        if (key == 'edge') 'edges': [value] else key: value,
      'reversed': false,
    };
    final List<double> xy;
    try {
      xy = widget.engine.sewSidePoints(sideJson: jsonEncode(stored));
    } on ApiError {
      return const []; // painting: nothing to show, nothing to report
    }
    return [
      for (var i = 0; i + 1 < xy.length; i += 2) Offset(xy[i], xy[i + 1]),
    ];
  }

  /// The Sew tool's picks: the track under the pointer, the edges
  /// ⌘-clicked, the first side and the stretch being dragged.
  void _paintSew(
    Canvas canvas,
    Offset Function(Offset mm) toScreen,
    Palette p,
  ) {
    Paint pen(double alpha, double width) => Paint()
      ..color = p.accent.withValues(alpha: alpha)
      ..strokeWidth = width
      ..strokeCap = StrokeCap.round
      ..style = PaintingStyle.stroke;
    void stroke(List<Offset> mm, Paint paint) {
      if (mm.length < 2) return;
      final pts = [for (final q in mm) toScreen(q)];
      final path = Path()..moveTo(pts.first.dx, pts.first.dy);
      for (final s in pts.skip(1)) {
        path.lineTo(s.dx, s.dy);
      }
      canvas.drawPath(path, paint);
    }

    Map<String, Object> whole(String track) => {
      track.startsWith('line:') ? 'line' : 'edge': track,
    };
    final drag = _sewDrag;
    if (_sewHover case final h? when drag == null) {
      stroke(_sidePoints(whole(h.track)), pen(0.35, 5));
    }
    for (final e in _sewChain) {
      stroke(_sidePoints({'edge': e}), pen(0.7, 4));
    }
    if (_sewFirst case final first?) {
      stroke(_sidePoints(first), pen(1, 4));
    }
    final dot = Paint()..color = p.accent;
    if (drag case (final a, final b, _)) {
      final (from, to) = a.offsetMm < b.offsetMm
          ? (a.offsetMm, b.offsetMm)
          : (b.offsetMm, a.offsetMm);
      stroke(
        _sidePoints({...whole(a.track), 'from_mm': from, 'to_mm': to}),
        pen(1, 4),
      );
      for (final s in [a, b]) {
        canvas.drawCircle(toScreen(Offset(s.xMm, s.yMm)), 4, dot);
      }
    } else if (_sewHover case final h?) {
      canvas.drawCircle(toScreen(Offset(h.xMm, h.yMm)), 4, dot);
    }
  }

  void _addRectangle() {
    final panels = (_summary['panels'] as List<dynamic>? ?? const []);
    var right = 0.0;
    for (final p in panels) {
      final max = (p['bbox']?['max'] as List<dynamic>?)?.first as num?;
      if (max != null && max > right) right = max.toDouble();
    }
    _shapeAdded(
      _run({
        'type': 'panel.create_rect',
        'name': context.l.pieceNew(panels.length + 1),
        'origin': [right + 60, 0],
        'width': 300,
        'height': 400,
      }),
    );
  }

  /// Ask where to save, then export there: the pattern, or the garment as
  /// the 3D view shows it.
  Future<void> _export(ExportFormat format, bool avatar) async {
    final ext = switch (format) {
      ExportFormat.svg => 'svg',
      ExportFormat.pdfA4 || ExportFormat.pdfSingle => 'pdf',
      ExportFormat.glb => 'glb',
      ExportFormat.obj => 'obj',
      ExportFormat.dxf => 'dxf',
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
      await runBusy(
        context,
        widget.engine,
        l.export,
        () => widget.engine.exportFile(
          format: format,
          path: location.path,
          avatar: avatar,
        ),
      );
      if (mounted) {
        _toast(l.exported(location.path.split(RegExp(r'[/\\]')).last));
      }
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
    }
  }

  /// The marker dialog: the pieces laid out on the fabric.
  Future<void> _openMarker() async {
    final indicators = _run({'type': 'production.indicators'});
    await showDialog<void>(
      context: context,
      builder: (_) => _MarkerDialog(
        engine: widget.engine,
        summary: _summary,
        indicators: indicators is Map<String, dynamic> ? indicators : null,
      ),
    );
  }

  /// The game-engine export dialog; a toast once the file is written.
  Future<void> _openGameExport() async {
    final saved = await showDialog<String>(
      context: context,
      builder: (_) => _GameExportDialog(
        engine: widget.engine,
        name: (_summary['name'] as String?) ?? 'Rustle3D',
      ),
    );
    if (saved != null && mounted) _toast(context.l.exported(saved));
  }

  /// Ask where to save, then write the tech pack: the garment as the 3D
  /// view shows it now, front and back, then the production details, in
  /// the user's language.
  Future<void> _exportTechpack() async {
    final name = (_summary['name'] as String?) ?? 'Rustle3D';
    final l = context.l;
    final location = await getSaveLocation(
      suggestedName: '$name-techpack.pdf',
      acceptedTypeGroups: [
        const XTypeGroup(label: 'PDF', extensions: ['pdf']),
      ],
    );
    if (location == null || !mounted) return;
    try {
      await runBusy(
        context,
        widget.engine,
        l.export,
        () => widget.engine.exportTechpack(
          path: location.path,
          labelsJson: jsonEncode(techpackLabels(l)),
          letter: false,
        ),
      );
      if (mounted) {
        _toast(l.exported(location.path.split(RegExp(r'[/\\]')).last));
      }
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
    }
  }

  /// The pattern at 1:1 for a projector over the cutting table: the
  /// pieces selected (all when none is), in the size shown in 3D.
  Future<void> _openProjector() async {
    final selection = widget.engine.selection();
    final String drawing;
    try {
      drawing = widget.engine.projectorDrawing(panels: selection);
    } on ApiError catch (e) {
      _toast(context.error(e));
      return;
    }
    await Navigator.of(context).push(
      PageRouteBuilder<void>(
        opaque: true,
        pageBuilder: (_, _, _) => ProjectorPage(
          drawing: jsonDecode(drawing) as Map<String, dynamic>,
          memory: ProjectorMemory(widget.libraryDir),
        ),
      ),
    );
  }

  /// Ask for a pattern file and import its pieces. When the file does not
  /// say how big its units are, ask which unit it was drawn in and import
  /// it again in that unit (the first import is undone).
  Future<void> _importPattern() async {
    final l = context.l;
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(
          label: l.importFiles,
          extensions: const [
            'dxf',
            'svg',
            'pdf',
            'sm2d',
            'val',
            'smis',
            'smms',
            'vit',
            'vst',
          ],
        ),
      ],
    );
    if (file == null || !mounted) return;
    Map<String, dynamic>? result = await _importFile(file.path, null);
    if (result == null || !mounted) return;
    if (result['scale'] == 'assumed') {
      final unit = await _askUnits();
      if (unit != null && mounted) {
        _run({'type': 'history.undo'});
        result = await _importFile(file.path, unit) ?? result;
      }
    }
    if (!mounted) return;
    _refresh();
    // A measurement file gives the avatar's body.
    if (result['panels'] == null) {
      _toast(l.measurementsImported);
      return;
    }
    final pieces = (result['panels'] as List<dynamic>).length;
    final ignored = result['ignored'] as int? ?? 0;
    final unsupported = (result['unsupported'] as List<dynamic>?)?.length ?? 0;
    _toast(
      [
        l.imported(pieces),
        if (unsupported > 0)
          l.importUnsupported(unsupported)
        else if (ignored > 0)
          l.importIgnored(ignored),
      ].join(' '),
    );
  }

  Future<Map<String, dynamic>?> _importFile(String path, String? units) async {
    try {
      final out = await runBusy(
        context,
        widget.engine,
        context.l.importPattern,
        () => widget.engine.importPattern(path: path, units: units),
      );
      return jsonDecode(out) as Map<String, dynamic>;
    } on ApiError catch (e) {
      if (mounted) _toast(context.error(e));
      return null;
    }
  }

  Future<String?> _askUnits() {
    final l = context.l;
    return showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l.importUnitsTitle),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(l.importUnitsBody),
            const SizedBox(height: Space.s),
            for (final (unit, label) in [
              ('mm', l.unitMm),
              ('cm', l.unitCm),
              ('in', l.unitIn),
              ('pt', l.unitPt),
              ('px', l.unitPx),
            ])
              ListTile(
                dense: true,
                title: Text(label),
                onTap: () => Navigator.pop(context, unit),
              ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(l.importKeep),
          ),
        ],
      ),
    );
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

  // Undo, redo and going back in the history select what changed.
  void _undo() {
    if (_historyAt == 0) return;
    _run({'type': 'history.undo'});
    _refresh();
  }

  void _redo() {
    if (_historyAt == _history.length) return;
    _run({'type': 'history.redo'});
    _refresh();
  }

  /// Copy the selected pieces, with what belongs to them (seams between
  /// them, notches, lines, pins, grade rules), as a clip on the system
  /// clipboard: it pastes here or in another project.
  Future<void> _copy() async {
    final l = context.l;
    if (_selection.isEmpty) return _toast(l.selectToCopy);
    final clip = _run({'type': 'panel.copy', 'panels': _selection});
    if (clip == null) return;
    await Clipboard.setData(ClipboardData(text: jsonEncode(clip)));
    if (mounted) _toast(context.keys(l.copied(_selection.length)));
  }

  /// Paste the pieces of a clip, and select them.
  Future<void> _paste() async {
    final l = context.l;
    final text = (await Clipboard.getData(Clipboard.kTextPlain))?.text;
    if (!mounted) return;
    Object? clip;
    try {
      clip = text == null ? null : jsonDecode(text);
    } on FormatException {
      clip = null;
    }
    if (clip is! Map || clip['format'] != 'rustle3d.clip') {
      return _toast(context.keys(l.nothingToPaste));
    }
    final out = _run({'type': 'panel.paste', 'clip': clip});
    if (out == null) return;
    final panels = [
      for (final id in (out['created'] as List<dynamic>))
        if ((id as String).startsWith('panel:')) id,
    ];
    _guard(() => widget.engine.select(panels: panels));
    _refresh();
    _toast(l.pasted(panels.length));
  }

  void _goToStep(int at) {
    _run({'type': 'history.go_to', 'at': at});
    _refresh();
  }

  Future<void> _deleteSelected() async {
    // Construction lines, notches, or a single corner, go without asking.
    if (_guides.isNotEmpty || _notches.isNotEmpty) {
      _run({
        'type': 'batch',
        'commands': [
          for (final g in _guides) {'type': 'guide.remove', 'guide': g},
          for (final n in _notches) {'type': 'notch.remove', 'notch': n.id},
        ],
      });
      return _refresh();
    }
    if (_tool == 'select' && _points.length == 1 && _edges.isEmpty) {
      _run({'type': 'point.remove', 'point': _points.first.id});
      return _refresh();
    }
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
      s.backend.toUpperCase(),
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
        SingleActivator(LogicalKeyboardKey.keyC, meta: isMac, control: !isMac):
            _copy,
        SingleActivator(LogicalKeyboardKey.keyV, meta: isMac, control: !isMac):
            _paste,
        const SingleActivator(LogicalKeyboardKey.keyR): _addRectangle,
        const SingleActivator(LogicalKeyboardKey.keyV): () =>
            _setTool('select'),
        const SingleActivator(LogicalKeyboardKey.keyD): () => _setTool('draw'),
        const SingleActivator(LogicalKeyboardKey.keyS): () => _setTool('sew'),
        const SingleActivator(LogicalKeyboardKey.keyP): () =>
            _setTool('polygon'),
        const SingleActivator(LogicalKeyboardKey.keyC): () => _setTool('curve'),
        const SingleActivator(LogicalKeyboardKey.keyN): () => _setTool('notch'),
        const SingleActivator(LogicalKeyboardKey.keyB): () => _setTool('bend'),
        const SingleActivator(LogicalKeyboardKey.keyL): () =>
            _setTool('straighten'),
        const SingleActivator(LogicalKeyboardKey.keyA): () =>
            _setTool('addPoint'),
        const SingleActivator(LogicalKeyboardKey.keyK): () => _setTool('cut'),
        const SingleActivator(LogicalKeyboardKey.keyG): () => _setTool('guide'),
        const SingleActivator(LogicalKeyboardKey.keyT): () =>
            _setTool('dimension'),
        const SingleActivator(LogicalKeyboardKey.keyE): _addEllipse,
        const SingleActivator(LogicalKeyboardKey.enter): _finish,
        const SingleActivator(LogicalKeyboardKey.escape): () => setState(() {
          _polygon.clear();
          _path.clear();
          _guideStart = null;
          _sewFirst = null;
          _sewChain.clear();
          _dimPicks.clear();
        }),
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
        const SingleActivator(LogicalKeyboardKey.keyM): () =>
            _toggleMaximized(_hovered),
      },
      child: Focus(
        focusNode: _keys,
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
                history: _history,
                historyAt: _historyAt,
                onUndo: _undo,
                onRedo: _redo,
                onGoTo: _goToStep,
                onBody: _onBodyMenu,
                bodyCheck: _bodyCheck,
                showPoints: _showPoints,
                onExport: _export,
                onProduction: (what) => switch (what) {
                  'marker' => _openMarker(),
                  'techpack' => _exportTechpack(),
                  'game' => _openGameExport(),
                  _ => _openProjector(),
                },
                onImport: _importPattern,
                onLibrary: _showPatternLibrary,
                agent: _agent,
                onAgents: () => showAgents(
                  context,
                  engine: widget.engine,
                  onAllow: _allowAgents,
                  onDisconnect: () => _guard(widget.engine.disconnectAgent),
                ),
                bodyKey: _spotBody,
                onLearn: () => _scaffold.currentState?.openEndDrawer(),
                onHome: widget.onHome,
              ),
              _ActionBar(picked: _pickedText(l), groups: _barActions(l)),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(Space.s, 0, Space.m, 0),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _ToolRail(
                        active: _tool,
                        shown: _groupShown,
                        keys: _spotTools,
                        onSelect: (tool) {
                          for (final (group, tools) in _ToolRail.groups) {
                            if (tools.contains(tool)) {
                              setState(() => _groupShown[group] = tool);
                            }
                          }
                          switch (tool) {
                            case 'rectangle':
                              _addRectangle();
                            case 'ellipse':
                              _addEllipse();
                            case 'sector':
                              _addSector();
                            default:
                              _setTool(tool);
                          }
                        },
                      ),
                      if (_maximized != ViewKind.garment)
                        Expanded(
                          child: MouseRegion(
                            onEnter: (_) => _hovered = ViewKind.board,
                            child: Listener(
                              onPointerDown: (_) => _keys.requestFocus(),
                              child: _ViewCard(
                                key: _tourPattern,
                                title: l.viewPattern,
                                icon: Icons.architecture_outlined,
                                maximized: _maximized == ViewKind.board,
                                onToggleSize: () =>
                                    _toggleMaximized(ViewKind.board),
                                child: Stack(
                                  children: [
                                    Positioned.fill(
                                      child: Listener(
                                        // Where a dimension's value is typed.
                                        onPointerDown: (e) =>
                                            _boardDown = e.localPosition,
                                        child: RustView(
                                          engine: widget.engine,
                                          kind: ViewKind.board,
                                          // Corners and edges are dragged with Select.
                                          holding: _tool == 'select',
                                          drawing: _tool == 'draw',
                                          onDrawn: (_) {
                                            _refresh();
                                            _toast(l.toastLineAdded);
                                          },
                                          pattern: switch (_tool) {
                                            'polygon' => PatternPointer(
                                              onTap: _polygonTap,
                                              onHover: _hoverAt,
                                            ),
                                            'guide' => PatternPointer(
                                              onTap: _guideTap,
                                              onHover: _hoverAt,
                                            ),
                                            'arc' => PatternPointer(
                                              onTap: _arcTap,
                                            ),
                                            'dart' => PatternPointer(
                                              onTap: _dartTap,
                                            ),
                                            'pleat' => PatternPointer(
                                              onTap: _pleatTap,
                                            ),
                                            'button' => PatternPointer(
                                              onTap: _buttonTap,
                                            ),
                                            'zipper' => PatternPointer(
                                              onTap: _zipperTap,
                                            ),
                                            'hole' => PatternPointer(
                                              onTap: _polygonTap,
                                              onHover: _hoverAt,
                                            ),
                                            'notch' => PatternPointer(
                                              onTap: _notchTap,
                                            ),
                                            'curve' || 'cut' => PatternPointer(
                                              onPress: _pathPress,
                                              onDrag: _pathDrag,
                                              onHover: _hoverAt,
                                            ),
                                            'bend' => PatternPointer(
                                              onTap: _bendTap,
                                              onPress: _handlePress,
                                              onDrag: _handleDrag,
                                              onRelease: _handleRelease,
                                            ),
                                            'select' => PatternPointer(
                                              onPress: (mm, k) =>
                                                  _labelPress(mm) ||
                                                  _framePress(mm, k) ||
                                                  _handlePress(mm, k),
                                              onDrag: (mm) => _labelDrag != null
                                                  ? _labelDragTo(mm)
                                                  : _frameDrag != null
                                                  ? _frameDragTo(mm)
                                                  : _handleDrag(mm),
                                              onRelease: () {
                                                _labelRelease();
                                                _frameRelease();
                                                _handleRelease();
                                              },
                                            ),
                                            'straighten' => PatternPointer(
                                              onTap: _straightenTap,
                                            ),
                                            'dimension' => PatternPointer(
                                              onTap: _dimTap,
                                              onHover: _dimHoverAt,
                                            ),
                                            'sew' => PatternPointer(
                                              onPress: _sewPress,
                                              onDrag: _sewDragTo,
                                              onRelease: _sewRelease,
                                              onHover: _sewHoverAt,
                                            ),
                                            'addPoint' => PatternPointer(
                                              onTap: _addPointTap,
                                            ),
                                            _ => null,
                                          },
                                          overlay: (canvas, toScreen) =>
                                              _boardOverlay(
                                                canvas,
                                                toScreen,
                                                Tokens.of(context),
                                              ),
                                          onSelection: (_) => _refresh(),
                                          onFrameTime: (ms) =>
                                              _onFrame(true, ms),
                                        ),
                                      ),
                                    ),
                                    if (_dimField case (final c, final at))
                                      Positioned(
                                        left: at.dx - 55,
                                        top: at.dy + 12,
                                        width: 110,
                                        child: DecoratedBox(
                                          decoration: BoxDecoration(
                                            color: Tokens.of(context).surface,
                                            borderRadius: BorderRadius.circular(
                                              6,
                                            ),
                                          ),
                                          child: _DimensionField(
                                            key: ValueKey('placed-${c['id']}'),
                                            constraint: c,
                                            autofocus: true,
                                            run: (step) {
                                              _run(step);
                                              _refresh();
                                            },
                                            onDone: () => setState(
                                              () => _dimField = null,
                                            ),
                                          ),
                                        ),
                                      ),
                                    // Top right: the view's title is top left.
                                    if (_tool == 'polygon' || _tool == 'curve')
                                      Positioned(
                                        right: Space.m,
                                        top: Space.m,
                                        child: Tooltip(
                                          message: l.symmetricHalfTip,
                                          // Enter still closes the piece.
                                          child: ExcludeFocus(
                                            child: FilterChip(
                                              avatar: const Icon(
                                                Icons.flip_rounded,
                                                size: 16,
                                              ),
                                              label: Text(l.symmetricHalf),
                                              selected: _symmetric,
                                              onSelected: (on) => setState(
                                                () => _symmetric = on,
                                              ),
                                            ),
                                          ),
                                        ),
                                      ),
                                  ],
                                ),
                              ),
                            ),
                          ),
                        ),
                      if (_maximized == null) const SizedBox(width: Space.m),
                      if (_maximized != ViewKind.board)
                        Expanded(
                          child: MouseRegion(
                            onEnter: (_) => _hovered = ViewKind.garment,
                            child: Listener(
                              onPointerDown: (_) => _keys.requestFocus(),
                              child: _ViewCard(
                                key: _tourGarment,
                                title: _photo == null ? '3D' : l.photo,
                                maximized: _maximized == ViewKind.garment,
                                onToggleSize: () =>
                                    _toggleMaximized(ViewKind.garment),
                                icon: _photo == null
                                    ? Icons.view_in_ar_outlined
                                    : Icons.photo_camera_outlined,
                                trailing: _photo != null
                                    ? null
                                    : Row(
                                        mainAxisSize: MainAxisSize.min,
                                        children: [
                                          _CameraBar(
                                            key: _spotCameras,
                                            onLook: _look,
                                          ),
                                          const SizedBox(width: Space.s),
                                          _SimControls(
                                            key: _spotSimulate,
                                            status: _sim,
                                            onRun: _setRunning,
                                            onRestart: _restart,
                                            onMotion: _setMotion,
                                            captured: () {
                                              try {
                                                return widget.engine.motions();
                                              } on ApiError {
                                                return const <String>[];
                                              }
                                            },
                                            library: () => libraryMotions(
                                              libraryDir: widget.libraryDir,
                                            ),
                                            onLibraryMotion: _useLibraryMotion,
                                            onImportMotion: _importMotion,
                                            onMotionLibrary: _showMotionLibrary,
                                          ),
                                          const SizedBox(width: Space.s),
                                          _FitButton(
                                            map: _fitMap,
                                            onMap: _setFitMap,
                                          ),
                                          const SizedBox(width: Space.s),
                                          _Pill(
                                            key: _spotPhoto,
                                            child: _IconAction(
                                              key: const ValueKey(
                                                'photoButton',
                                              ),
                                              icon: Icons.photo_camera_outlined,
                                              tip: l.photoTip,
                                              onTap: _askPhoto,
                                            ),
                                          ),
                                        ],
                                      ),
                                overlay: _photo == null
                                    ? null
                                    : PhotoBar(
                                        status: _photo!,
                                        onStop: _stopPhoto,
                                        onSave: _savePhoto,
                                        onClose: _closePhoto,
                                      ),
                                child: Stack(
                                  children: [
                                    Positioned.fill(
                                      child: RustView(
                                        engine: widget.engine,
                                        kind: ViewKind.garment,
                                        onSelection: (_) => _refresh(),
                                        onFrameTime: (ms) =>
                                            _onFrame(false, ms),
                                        onPinned: (_) {
                                          _refresh();
                                          _toast(l.toastPinned);
                                        },
                                        drawing: _tool == 'draw',
                                        onDrawn: (_) {
                                          _refresh();
                                          _toast(l.toastLineAdded);
                                        },
                                        onMeasure: _tool == 'tape'
                                            ? _measure
                                            : null,
                                        bodyPoints: _showPoints,
                                        onBodyPoint: _refresh,
                                      ),
                                    ),
                                    if (_tool == 'tape')
                                      // Below the view's title and its
                                      // controls.
                                      Positioned(
                                        left: Space.m,
                                        top: 64,
                                        child: _TapeCard(
                                          length: _tapeLength,
                                          onLength: (length) => setState(() {
                                            _clearTape();
                                            _tapeLength = length;
                                          }),
                                          reading: _tape,
                                          started: _lengthFrom != null,
                                          lengthReading: _length,
                                          onClose: () => _setTool('select'),
                                        ),
                                      ),
                                    if (_fitLegend case final legend?)
                                      // Above the view's size switch.
                                      Positioned(
                                        right: Space.m,
                                        bottom: 64,
                                        child: _FitLegendCard(legend: legend),
                                      ),
                                    if (_fitting)
                                      const Positioned(
                                        left: Space.m,
                                        bottom: Space.m,
                                        child: _FittingChip(),
                                      ),
                                  ],
                                ),
                              ),
                            ),
                          ),
                        ),
                      const SizedBox(width: Space.m),
                      SizedBox(
                        width: 264,
                        child: _Inspector(
                          key: _spotInspector,
                          engine: widget.engine,
                          selection: _selection,
                          points: _points,
                          edges: _edges,
                          constraints: _constraints,
                          constraintGroups: _constraintGroups,
                          panel: _panel,
                          summary: _summary,
                          edge: _edge,
                          seams: _seams,
                          pins: _pins,
                          onUnpin: (pin) {
                            _guard(() => widget.engine.unpin(pin: pin));
                            _refresh();
                          },
                          onArc: _askArc,
                          run: (step) async {
                            if (_longSteps.contains(step['type'])) {
                              await _runLong(step);
                            } else {
                              _run(step);
                            }
                            if (!mounted) return;
                            _refresh();
                            // A new placement, or another size on its own
                            // body, starts the drape again from around the
                            // body.
                            if (step['type'] == 'panel.place' ||
                                step['type'] == 'grading.show_size' ||
                                step['type'] == 'grading.use_chart') {
                              await _restart();
                            }
                          },
                          query: _run,
                          onLibrary: _showPatternLibrary,
                          showSizes: widget.engine.showSizes(),
                          onShowSizes: (show) => setState(
                            () => widget.engine.setShowSizes(show_: show),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              _StatusBar(
                hint: switch (_tool) {
                  'sew' when _sewChain.isNotEmpty => context.keys(l.hintSewRun),
                  'sew' when _sewFirst == null => context.keys(l.hintSewFirst),
                  'sew' => context.keys(l.hintSewSecond),
                  'draw' => l.hintDraw,
                  'tape' => l.hintTape,
                  'polygon' => l.hintPolygon,
                  'curve' => l.hintCurve,
                  'bend' => l.hintBend,
                  'straighten' => l.hintStraighten,
                  'addPoint' => l.hintAddPoint,
                  'arc' => l.hintArc,
                  'dart' => l.hintDart,
                  'pleat' => l.hintPleat,
                  'hole' => l.hintHole,
                  'guide' => l.hintGuide,
                  'dimension' when _dimPicks.isEmpty => l.hintDimension,
                  'dimension' => l.hintDimensionPlace,
                  'cut' => l.hintCut,
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
    required this.history,
    required this.historyAt,
    required this.onUndo,
    required this.onGoTo,
    required this.onRedo,
    required this.onBody,
    required this.bodyCheck,
    required this.showPoints,
    required this.onExport,
    required this.onProduction,
    required this.onImport,
    required this.onLibrary,
    required this.agent,
    required this.onAgents,
    required this.bodyKey,
    required this.onLearn,
    required this.onHome,
  });

  final String projectName;
  final bool dark;
  final VoidCallback onToggleTheme;
  final String language;
  final ValueChanged<String> onLanguage;

  /// The undo history (command names, oldest first) and the steps done.
  final List<String> history;
  final int historyAt;
  final VoidCallback onUndo;
  final VoidCallback onRedo;

  /// Go back or forward until this many steps are done.
  final ValueChanged<int> onGoTo;

  /// A choice of the Body menu (see `_onBodyMenu`).
  final ValueChanged<String> onBody;

  /// The imported body's check; `null` with the built-in body.
  final Map<String, dynamic>? bodyCheck;

  /// The arrangement points are shown in the 3D view.
  final bool showPoints;

  /// Export in a format (with the avatar, for 3D formats).
  final void Function(ExportFormat format, bool avatar) onExport;

  /// A production export: `marker` (the pieces laid out on the fabric),
  /// `techpack` (the tech pack PDF) or `projector` (the pattern at 1:1 on
  /// a projector).
  final ValueChanged<String> onProduction;

  /// Import a pattern drawn elsewhere.
  final VoidCallback onImport;

  /// The pattern library: add a pattern, manage the user's own.
  final VoidCallback onLibrary;

  /// AI agents: allowed or not, connected or not.
  final AgentStatus agent;
  final VoidCallback onAgents;

  /// The Body button, for Learn to point at.
  final GlobalKey bodyKey;
  final VoidCallback onLearn;
  final VoidCallback onHome;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
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
            const Spacer(),
            TextButton.icon(
              onPressed: onLearn,
              icon: const Icon(Icons.school_outlined, size: 18),
              label: Text(l.learn),
            ),
            TextButton.icon(
              key: const ValueKey('libraryButton'),
              onPressed: onLibrary,
              icon: const Icon(Icons.collections_bookmark_outlined, size: 18),
              label: Text(l.libraryManage),
            ),
            _BodyMenu(
              key: bodyKey,
              check: bodyCheck,
              showPoints: showPoints,
              onChoice: onBody,
            ),
            _AgentButton(status: agent, onOpen: onAgents),
            Tooltip(
              message: l.importTip,
              child: TextButton.icon(
                onPressed: onImport,
                icon: const Icon(Icons.file_open_outlined, size: 18),
                label: Text(l.importPattern),
              ),
            ),
            PopupMenuButton<(ExportFormat?, bool, String)>(
              tooltip: l.export,
              onSelected: (choice) {
                if (choice.$1 case final format?) {
                  onExport(format, choice.$2);
                } else {
                  onProduction(choice.$3);
                }
              },
              itemBuilder: (_) => [
                for (final (format, avatar, label) in [
                  (ExportFormat.svg, false, l.exportSvg),
                  (ExportFormat.pdfA4, false, l.exportPdfA4),
                  (ExportFormat.pdfSingle, false, l.exportPdfSingle),
                  (ExportFormat.dxf, false, l.exportDxf),
                  (ExportFormat.glb, false, l.exportGlb),
                  (ExportFormat.glb, true, l.exportGlbAvatar),
                  (ExportFormat.obj, false, l.exportObj),
                ])
                  PopupMenuItem(
                    value: (format, avatar, ''),
                    child: Text(label),
                  ),
                PopupMenuItem(
                  key: const Key('export-game'),
                  value: (null, false, 'game'),
                  child: Text(l.exportGame),
                ),
                const PopupMenuDivider(),
                for (final (what, label) in [
                  ('marker', l.exportMarker),
                  ('techpack', l.exportTechpack),
                  ('projector', l.exportProjector),
                ])
                  PopupMenuItem(
                    key: Key('export-$what'),
                    value: (null, false, what),
                    child: Text(label),
                  ),
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
              tip: historyAt > 0
                  ? context.keys(
                      l.undoStep(context.stepName(history[historyAt - 1])),
                    )
                  : l.nothingToUndo,
              onTap: historyAt > 0 ? onUndo : null,
            ),
            _IconAction(
              icon: Icons.redo_rounded,
              tip: historyAt < history.length
                  ? context.keys(
                      l.redoStep(context.stepName(history[historyAt])),
                    )
                  : l.nothingToRedo,
              onTap: historyAt < history.length ? onRedo : null,
            ),
            _HistoryButton(steps: history, at: historyAt, onGoTo: onGoTo),
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

/// The top bar's Body button: measurements, an imported body, its check
/// and its arrangement points. A dot says the imported body has
/// something to check.
class _BodyMenu extends StatelessWidget {
  const _BodyMenu({
    super.key,
    required this.check,
    required this.showPoints,
    required this.onChoice,
  });

  final Map<String, dynamic>? check;
  final bool showPoints;
  final ValueChanged<String> onChoice;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final imported = check != null;
    final concern = checkConcern(check);
    return PopupMenuButton<String>(
      tooltip: l.body,
      onSelected: onChoice,
      itemBuilder: (_) => [
        PopupMenuItem(
          value: 'measurements',
          child: Text(l.bodyMenuMeasurements),
        ),
        PopupMenuItem(
          key: const Key('bodyMenuImport'),
          value: 'import',
          child: Text(l.bodyMenuImport),
        ),
        if (imported) ...[
          const PopupMenuDivider(),
          PopupMenuItem(
            key: const Key('bodyMenuCheck'),
            value: 'check',
            child: Text(l.bodyMenuCheck),
          ),
          CheckedPopupMenuItem(
            key: const Key('bodyMenuPoints'),
            value: 'points',
            checked: showPoints,
            child: Text(l.bodyMenuPoints),
          ),
          PopupMenuItem(value: 'builtin', child: Text(l.bodyMenuBuiltin)),
        ],
      ],
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: Space.s, vertical: 6),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Badge(
              isLabelVisible: concern != null,
              backgroundColor: p.danger,
              smallSize: 7,
              child: Icon(
                Icons.accessibility_new_rounded,
                size: 18,
                color: p.accent,
              ),
            ),
            const SizedBox(width: 6),
            Text(
              l.body,
              style: ts(
                fontSize: 14,
                fontWeight: FontWeight.w500,
                color: p.accent,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Says the body is being fitted to new measurements, over the 3D view
/// (which shows the old body until then).
class _FittingChip extends StatelessWidget {
  const _FittingChip();

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: Space.m, vertical: 6),
      decoration: BoxDecoration(
        color: p.surface.withValues(alpha: 0.92),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            width: 14,
            height: 14,
            child: CircularProgressIndicator(strokeWidth: 2, color: p.accent),
          ),
          const SizedBox(width: Space.s),
          Text(
            context.l.bodyFitting,
            style: ts(fontSize: 12, color: p.textMuted),
          ),
        ],
      ),
    );
  }
}

/// The undo history as a list: the steps done, then the undone ones
/// (muted); a click goes back or forward to just after that step.
class _HistoryButton extends StatelessWidget {
  const _HistoryButton({
    required this.steps,
    required this.at,
    required this.onGoTo,
  });

  final List<String> steps;
  final int at;
  final ValueChanged<int> onGoTo;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    Widget row(int target, String label, IconData icon) {
      final current = target == at;
      final undone = target > at;
      return MenuItemButton(
        onPressed: current ? null : () => onGoTo(target),
        leadingIcon: Icon(
          current ? Icons.arrow_right_rounded : icon,
          size: 18,
          color: current ? p.accent : p.textMuted,
        ),
        child: Text(
          label,
          style: ts(
            fontSize: 13,
            fontWeight: current ? FontWeight.w600 : FontWeight.w400,
            color: current
                ? p.text
                : undone
                ? p.textMuted.withValues(alpha: 0.6)
                : p.text,
          ),
        ),
      );
    }

    return MenuAnchor(
      menuChildren: [
        Padding(
          padding: const EdgeInsets.fromLTRB(Space.m, Space.s, Space.m, 4),
          child: Text(
            l.historyTitle,
            style: ts(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: p.textMuted,
            ),
          ),
        ),
        ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 420, minWidth: 220),
          child: SingleChildScrollView(
            // The latest steps first: the ones a user goes back to.
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (var i = steps.length; i >= 1; i--)
                  row(i, context.stepName(steps[i - 1]), Icons.circle_outlined),
                row(0, l.historyStart, Icons.folder_open_outlined),
              ],
            ),
          ),
        ),
      ],
      builder: (context, menu, _) => _IconAction(
        icon: Icons.history_rounded,
        tip: l.historyTip,
        onTap: () => menu.isOpen ? menu.close() : menu.open(),
      ),
    );
  }
}

class _IconAction extends StatelessWidget {
  const _IconAction({
    super.key,
    required this.icon,
    required this.tip,
    required this.onTap,
  });

  final IconData icon;
  final String tip;

  /// `null` greys the action out.
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final color = onTap == null
        ? p.textMuted.withValues(alpha: 0.35)
        : p.textMuted;
    return Tooltip(
      message: tip,
      child: IconButton(
        onPressed: onTap,
        icon: Icon(icon, size: 19, color: color),
        style: IconButton.styleFrom(
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(Tokens.radius(context, 0.5)),
          ),
        ),
      ),
    );
  }
}

/// A point of a Curve or Cut path (mm): a corner, or a smooth point whose
/// curve leaves towards [handle] (and arrives from its mirror image).
class _Anchor {
  _Anchor(this.at, {Offset? handle})
    : handle = handle == null || (handle - at).distance < 1 ? null : handle;

  final Offset at;
  final Offset? handle;
  Offset? get handleIn => handle == null ? null : at * 2 - handle!;
}

/// A rail tool's tip: to the right of the button, centred on it, kept on
/// screen.
Offset _besideRail(TooltipPositionContext c) {
  final x = c.target.dx + c.targetSize.width / 2 + 8;
  final y = c.target.dy - c.tooltipSize.height / 2;
  final bottom = c.overlaySize.height - c.tooltipSize.height;
  return Offset(x, y.clamp(0, bottom < 0 ? 0 : bottom));
}

/// One rail tool: id, icon, name, key, hint.
typedef _RailTool = (String, IconData, String, String, String);

class _ToolRail extends StatelessWidget {
  const _ToolRail({
    required this.active,
    required this.shown,
    required this.keys,
    required this.onSelect,
  });

  final String active;

  /// The tool shown on each group's button: the last one used.
  final Map<String, String> shown;

  /// Buttons Learn points at, by tool.
  final Map<String, GlobalKey> keys;
  final ValueChanged<String> onSelect;

  /// The groups: one button, a menu of their tools.
  static const groups = [
    ('shapes', ['rectangle', 'ellipse', 'sector']),
    ('curves', ['bend', 'straighten', 'addPoint', 'arc']),
    ('details', ['dart', 'pleat']),
    ('trims', ['button', 'zipper']),
  ];

  static List<_RailTool> _details(L l) => [
    ('dart', Icons.change_history_rounded, l.toolDart, '', l.toolDartHint),
    ('pleat', Icons.view_week_outlined, l.toolPleat, '', l.toolPleatHint),
  ];

  static List<_RailTool> _trims(L l) => [
    (
      'button',
      Icons.radio_button_checked_rounded,
      l.toolButton,
      '',
      l.toolButtonHint,
    ),
    (
      'zipper',
      Icons.vertical_split_outlined,
      l.toolZipper,
      '',
      l.toolZipperHint,
    ),
  ];

  static List<_RailTool> _shapes(L l) => [
    (
      'rectangle',
      Icons.crop_square_rounded,
      l.toolRectangle,
      'R',
      l.toolRectangleHint,
    ),
    ('ellipse', Icons.circle_outlined, l.toolEllipse, 'E', l.toolEllipseHint),
    ('sector', Icons.donut_large_rounded, l.toolSector, '', l.toolSectorHint),
  ];

  static List<_RailTool> _curveEdits(L l) => [
    ('bend', Icons.gesture_rounded, l.toolBend, 'B', l.toolBendHint),
    (
      'straighten',
      Icons.horizontal_rule_rounded,
      l.toolStraighten,
      'L',
      l.toolStraightenHint,
    ),
    (
      'addPoint',
      Icons.add_circle_outline_rounded,
      l.toolAddPoint,
      'A',
      l.toolAddPointHint,
    ),
    ('arc', Icons.looks_rounded, l.toolArc, '', l.toolArcHint),
  ];

  /// The rail, top to bottom, in sections set apart by a line: drawing,
  /// measuring, sewing, trims. In a section, a list is a group (one
  /// button, a menu).
  static List<List<Object>> _sections(L l) => [
    [
      ('select', Icons.near_me_outlined, l.toolSelect, 'V', l.toolSelectHint),
      ('shapes', l.toolShapes, _shapes(l)),
      (
        'polygon',
        Icons.pentagon_outlined,
        l.toolPolygon,
        'P',
        l.toolPolygonHint,
      ),
      ('curve', Icons.draw_outlined, l.toolCurve, 'C', l.toolCurveHint),
      ('curves', l.toolEditCurve, _curveEdits(l)),
      ('cut', Icons.content_cut_rounded, l.toolCut, 'K', l.toolCutHint),
      (
        'hole',
        Icons.radio_button_unchecked_rounded,
        l.toolHole,
        '',
        l.toolHoleHint,
      ),
      ('draw', Icons.brush_rounded, l.toolDraw, 'D', l.toolDrawHint),
    ],
    [
      ('guide', Icons.linear_scale_rounded, l.toolGuide, 'G', l.toolGuideHint),
      (
        'dimension',
        Icons.square_foot_rounded,
        l.toolDimension,
        'T',
        l.toolDimensionHint,
      ),
      ('tape', Icons.straighten_rounded, l.toolTape, '', l.toolTapeHint),
    ],
    [
      (
        'notch',
        Icons.vertical_align_center_rounded,
        l.toolNotch,
        'N',
        l.toolNotchHint,
      ),
      ('details', l.toolDetails, _details(l)),
      ('sew', Icons.link_rounded, l.toolSew, 'S', l.toolSewHint),
    ],
    [('trims', l.toolTrims, _trims(l))],
  ];

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    TextSpan line(_RailTool t, {bool first = false}) {
      final (_, _, name, key, hint) = t;
      return TextSpan(
        children: [
          TextSpan(
            text: '${first ? '' : '\n\n'}$name  ',
            style: ts(fontWeight: FontWeight.w600),
          ),
          TextSpan(
            text: key,
            style: ts(color: p.surface.withValues(alpha: 0.6)),
          ),
          TextSpan(text: '\n$hint'),
        ],
      );
    }

    Widget button(
      _RailTool shown,
      String tip,
      InlineSpan message, {
      required bool active,
      required VoidCallback? onTap,
      void Function(BuildContext)? onMenu,
    }) {
      final (id, icon, _, _, _) = shown;
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Tooltip(
          richMessage: message,
          // Beside the rail, not over the next tool: the pointer can go
          // up and down the rail without the tip in the way.
          positionDelegate: _besideRail,
          child: Builder(
            builder: (context) => _RailButton(
              key: keys[id],
              icon: icon,
              label: tip,
              active: active,
              grouped: onMenu != null,
              onTap: onMenu == null ? onTap! : () => onMenu(context),
            ),
          ),
        ),
      );
    }

    return Container(
      width: 52,
      margin: const EdgeInsets.only(right: Space.s),
      padding: const EdgeInsets.symmetric(vertical: Space.s),
      decoration: BoxDecoration(
        color: p.surface,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        border: Border.all(color: p.border),
      ),
      // Scrolls in a short window rather than cut its last tools off.
      child: SingleChildScrollView(
        child: Column(
          children: [
            for (final (i, section) in _sections(l).indexed) ...[
              if (i > 0)
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: Space.s,
                    vertical: Space.xs,
                  ),
                  child: Divider(color: p.border, height: 1),
                ),
              for (final t in section)
                if (t is _RailTool)
                  button(
                    t,
                    t.$3,
                    line(t, first: true),
                    active: t.$1 == active,
                    onTap: () => onSelect(t.$1),
                  )
                else if (t case (
                  final String group,
                  final String name,
                  final List<_RailTool> tools,
                ))
                  button(
                    tools.firstWhere(
                      (x) => x.$1 == shown[group],
                      orElse: () => tools.first,
                    ),
                    name,
                    TextSpan(
                      children: [
                        TextSpan(
                          text: '$name\n',
                          style: ts(fontWeight: FontWeight.w700),
                        ),
                        for (final (i, x) in tools.indexed)
                          line(x, first: i == 0),
                      ],
                    ),
                    active: tools.any((x) => x.$1 == active),
                    onTap: null,
                    onMenu: (context) => _openGroup(context, tools),
                  ),
            ],
          ],
        ),
      ),
    );
  }

  /// The group's tools in a menu beside its button.
  Future<void> _openGroup(BuildContext context, List<_RailTool> group) async {
    final box = context.findRenderObject() as RenderBox?;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (box == null || overlay == null) return;
    final at = box.localToGlobal(
      box.size.topRight(const Offset(6, 0)),
      ancestor: overlay,
    );
    final p = Tokens.of(context);
    final picked = await showMenu<String>(
      context: context,
      position: RelativeRect.fromLTRB(
        at.dx,
        at.dy,
        overlay.size.width - at.dx,
        overlay.size.height - at.dy,
      ),
      items: [
        for (final (id, icon, name, key, _) in group)
          PopupMenuItem(
            value: id,
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 18,
                  color: id == active ? p.accent : p.textMuted,
                ),
                const SizedBox(width: Space.s),
                Expanded(child: Text(name)),
                const SizedBox(width: Space.m),
                Text(key, style: ts(color: p.textMuted)),
              ],
            ),
          ),
      ],
    );
    if (picked != null) onSelect(picked);
  }
}

class _RailButton extends StatefulWidget {
  const _RailButton({
    super.key,
    required this.icon,
    required this.label,
    required this.active,
    required this.grouped,
    required this.onTap,
  });

  final IconData icon;

  /// For screen readers and tests.
  final String label;
  final bool active;

  /// Opens a menu of tools: marked with a small corner.
  final bool grouped;
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
    final color = widget.active ? p.accent : p.textMuted;
    return Semantics(
      button: true,
      label: widget.label,
      child: MouseRegion(
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
            child: Stack(
              alignment: Alignment.center,
              children: [
                Icon(widget.icon, size: 20, color: color),
                if (widget.grouped)
                  Positioned(
                    right: 3,
                    bottom: 3,
                    child: CustomPaint(
                      size: const Size(5, 5),
                      painter: _CornerPainter(color),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// The small corner marking a button that opens more tools.
class _CornerPainter extends CustomPainter {
  const _CornerPainter(this.color);

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final path = Path()
      ..moveTo(size.width, 0)
      ..lineTo(size.width, size.height)
      ..lineTo(0, size.height)
      ..close();
    canvas.drawPath(path, Paint()..color = color);
  }

  @override
  bool shouldRepaint(_CornerPainter old) => old.color != color;
}

class _ViewCard extends StatelessWidget {
  const _ViewCard({
    super.key,
    required this.title,
    required this.icon,
    required this.child,
    required this.maximized,
    required this.onToggleSize,
    this.trailing,
    this.overlay,
  });

  final String title;
  final IconData icon;
  final Widget child;
  final Widget? trailing;

  /// The view takes the space of both views; [onToggleSize] switches.
  final bool maximized;
  final VoidCallback onToggleSize;

  /// Shown along the bottom; while it is, the view takes no input.
  final Widget? overlay;

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
          Positioned.fill(
            child: AbsorbPointer(absorbing: overlay != null, child: child),
          ),
          // The size switch in the bottom corner (the top is full on the
          // 3D view), over the overlay when there is one.
          Positioned(
            left: Space.m,
            right: Space.m,
            bottom: Space.m,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                _Pill(
                  child: _IconAction(
                    key: ValueKey(maximized ? 'restoreViews' : 'maximizeView'),
                    icon: maximized
                        ? Icons.close_fullscreen_rounded
                        : Icons.open_in_full_rounded,
                    tip: maximized
                        ? context.l.restoreViewsTip
                        : context.l.maximizeViewTip,
                    onTap: onToggleSize,
                  ),
                ),
                if (overlay != null) ...[
                  const SizedBox(height: Space.s),
                  overlay!,
                ],
              ],
            ),
          ),
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
    required this.engine,
    required this.selection,
    required this.points,
    required this.edges,
    required this.constraints,
    required this.constraintGroups,
    required this.onArc,
    required this.panel,
    required this.summary,
    required this.edge,
    required this.seams,
    required this.pins,
    required this.onUnpin,
    required this.run,
    required this.query,
    required this.showSizes,
    required this.onShowSizes,
    required this.onLibrary,
  });

  /// For the prints' previews and images.
  final Engine engine;

  /// Opens the pattern library.
  final VoidCallback onLibrary;
  final List<String> selection;
  final List<PickedPoint> points;

  /// Every edge picked, and the selected piece's constraints.
  final List<String> edges;
  final List<dynamic> constraints;
  final List<dynamic> constraintGroups;

  /// Make an edge an arc (asks how much it bulges): edge id, length (mm).
  final void Function(String edge, double lengthMm) onArc;
  final Map<String, dynamic>? panel;
  final Map<String, dynamic> summary;
  final String? edge;
  final List<dynamic> seams;
  final List<dynamic> pins;
  final ValueChanged<String> onUnpin;
  final ValueChanged<Map<String, Object?>> run;

  /// Run a query and return its result (null on error).
  final dynamic Function(Map<String, Object?>) query;

  /// Whether the board shows the other sizes' outlines.
  final bool showSizes;
  final ValueChanged<bool> onShowSizes;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final garments = (summary['garments'] as List<dynamic>?) ?? const [];
    final grading = summary['grading'] as Map<String, dynamic>?;
    final figures = query({'type': 'production.indicators'});
    final indicators = [
      if (figures is Map<String, dynamic>)
        for (final g in figures['garments'] as List) g as Map<String, dynamic>,
    ];
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
              engine: engine,
              summary: summary,
              indicators: indicators,
              run: run,
              onLibrary: onLibrary,
              query: query,
              showSizes: showSizes,
              onShowSizes: onShowSizes,
              onArrange: () => run({'type': 'garment.arrange'}),
              pins: pins,
              onUnpin: onUnpin,
            ),
            1 when panel != null => _PanelInspector(
              panel: panel!,
              trims: switch (query({'type': 'trim.list'})) {
                final Map<String, dynamic> t => t,
                _ => const {},
              },
              details: switch (query({'type': 'detail.list'})) {
                final Map<String, dynamic> d => d,
                _ => const {},
              },
              figures: [
                for (final g in indicators)
                  for (final f in g['pieces'] as List)
                    if (f['panel'] == panel!['id']) f as Map<String, dynamic>,
              ].firstOrNull,
              garments: garments,
              grading: grading,
              points: points,
              edges: edges,
              constraints: constraints,
              constraintGroups: constraintGroups,
              onArc: onArc,
              edge: edge,
              seams: seams,
              run: run,
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
    required this.engine,
    required this.summary,
    required this.indicators,
    required this.run,
    required this.onLibrary,
    required this.query,
    required this.showSizes,
    required this.onShowSizes,
    required this.onArrange,
    required this.pins,
    required this.onUnpin,
  });

  final Engine engine;
  final Map<String, dynamic> summary;

  /// `production.indicators`' garments: fabric details and totals.
  final List<Map<String, dynamic>> indicators;
  final ValueChanged<Map<String, Object?>> run;
  final VoidCallback onLibrary;
  final dynamic Function(Map<String, Object?>) query;
  final bool showSizes;
  final ValueChanged<bool> onShowSizes;
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
        _GarmentList(
          engine: engine,
          garments: (summary['garments'] as List<dynamic>?) ?? const [],
          indicators: indicators,
          run: run,
          onLibrary: onLibrary,
        ),
        const SizedBox(height: Space.xl),
        Divider(color: p.border, height: 1),
        const SizedBox(height: Space.l),
        _SizeRange(
          grading: summary['grading'] as Map<String, dynamic>?,
          run: run,
          query: query,
          showOnBoard: showSizes,
          onShowOnBoard: onShowSizes,
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
        _VariablesCard(query: query, run: run),
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
    required this.trims,
    required this.details,
    required this.garments,
    required this.grading,
    required this.points,
    required this.edges,
    required this.constraints,
    required this.constraintGroups,
    required this.onArc,
    required this.edge,
    required this.seams,
    required this.run,
    required this.figures,
  });

  /// Its `production.indicators` entry: cut area and mass.
  final Map<String, dynamic>? figures;

  /// Every button and zipper (`trim.list`).
  final Map<String, dynamic> trims;

  /// Every construction detail (`detail.list`).
  final Map<String, dynamic> details;

  final Map<String, dynamic> panel;

  /// The project's garments, to move the piece to another.
  final List<dynamic> garments;

  /// The project's size range (`project.summary`), if any.
  final Map<String, dynamic>? grading;
  final List<PickedPoint> points;
  final List<String> edges;
  final List<dynamic> constraints;
  final List<dynamic> constraintGroups;
  final void Function(String edge, double lengthMm) onArc;
  final String? edge;
  final List<dynamic> seams;
  final ValueChanged<Map<String, Object?>> run;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final id = panel['id'] as String;
    final linked = panel['mirror_of'] != null;
    final symmetric = panel['symmetry'] != null;
    final area = (panel['area_mm2'] as num) / 1e6;
    final perimeter = (panel['perimeter_mm'] as num) / 10;
    final allowance = (panel['seam_allowance_mm'] as num).toDouble();
    final grain = (panel['grain_angle_deg'] as num).toDouble();
    final l = context.l;
    return ListView(
      children: [
        Text(l.piece, style: ts(fontSize: 12, color: p.textMuted)),
        const SizedBox(height: Space.xs),
        _PieceName(
          key: ValueKey('piece-name-$id'),
          name: panel['name'] as String,
          onRename: (name) =>
              run({'type': 'panel.rename', 'panel': id, 'name': name}),
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
        if (symmetric) ...[
          const SizedBox(height: Space.s),
          _Badge(
            icon: Icons.flip_rounded,
            text: l.symmetricPiece,
            action: l.unlinkHalves,
            onAction: () => run({'type': 'panel.unlink', 'panel': id}),
          ),
        ],
        if ((panel['edges'] as List).firstWhere(
              (e) => e['id'] == edge,
              orElse: () => null,
            )
            case final Map<String, dynamic> e) ...[
          const SizedBox(height: Space.m),
          _EdgeCard(
            edge: e,
            seams: seams,
            run: run,
            // A straight edge of a whole piece can be its axis.
            unfold: linked || symmetric || e['curve']?['kind'] != 'line'
                ? null
                : () => run({
                    'type': 'panel.unfold',
                    'panel': id,
                    'edge': e['id'],
                  }),
            onArc: e['derived'] == true || linked
                ? null
                : () => onArc(
                    e['id'] as String,
                    (e['length_mm'] as num).toDouble(),
                  ),
          ),
        ],
        if (_TrimsCard.on(trims, id)) ...[
          const SizedBox(height: Space.m),
          _TrimsCard(panel: id, trims: trims, run: run),
        ],
        if (_DetailsCard.on(details, id)) ...[
          const SizedBox(height: Space.m),
          _DetailsCard(panel: id, details: details, run: run),
        ],
        if (points.isNotEmpty) ...[
          const SizedBox(height: Space.m),
          _PointsCard(
            points: points,
            run: run,
            grading: linked ? null : grading,
            panel: panel,
          ),
        ],
        if (!linked) ...[
          const SizedBox(height: Space.l),
          _ConstraintsCard(
            constraints: constraints,
            groups: constraintGroups,
            run: run,
          ),
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
        if (figures case final f?) ...[
          const SizedBox(height: Space.s),
          Row(
            children: [
              Expanded(
                child: _Stat(
                  label: l.cutArea,
                  value:
                      '${context.number((f['cut_area_mm2'] as num) / 1e6, 3)} m²',
                ),
              ),
              const SizedBox(width: Space.s),
              Expanded(
                child: _Stat(
                  label: l.pieceMass,
                  value: '${context.number(f['mass_g'] as num, 1)} g',
                ),
              ),
            ],
          ),
        ],
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
        // The fabric of the piece's garment, also shown with nothing
        // selected: changing it changes every piece of that garment.
        for (final g in garments)
          if (g['id'] == panel['garment']) ...[
            const SizedBox(height: Space.xl),
            _FabricPicker(
              current: g['fabric'] as String? ?? 'cotton',
              onPick: (fabric) => run({
                'type': 'garment.set_fabric',
                'garment': g['id'],
                'fabric': fabric,
              }),
            ),
          ],
        if (garments.any(
          (g) => g['id'] == panel['garment'] && g['print'] != null,
        )) ...[
          const SizedBox(height: Space.xl),
          _PiecePrint(panel: panel, run: run),
        ],
        if (garments.length > 1) ...[
          const SizedBox(height: Space.xl),
          DropdownButtonFormField<String>(
            initialValue: panel['garment'] as String?,
            decoration: InputDecoration(labelText: l.pieceGarment),
            items: [
              for (final g in garments)
                DropdownMenuItem(
                  value: g['id'] as String,
                  child: Text(g['name'] as String),
                ),
            ],
            onChanged: (to) =>
                run({'type': 'panel.set_garment', 'panel': id, 'garment': to}),
          ),
        ],
        const SizedBox(height: Space.xl),
        _PlacementCard(
          placement: panel['placement'] as Map<String, dynamic>?,
          place: (place) => run({'type': 'panel.place', 'panel': id, ...place}),
        ),
      ],
    );
  }
}

/// A turn or a scaling of the selected piece, dragged by a handle of its
/// frame: about `pivot` (its centre to turn, the opposite corner to
/// scale) from where the pointer pressed (`grab`); `corner` is the
/// bounding-box corner grabbed to scale, null to turn. What is applied so
/// far: `angle` (degrees) or `factor` (across, up and down).
class _FrameDrag {
  _FrameDrag(this.panel, this.pivot, this.grab, this.corner);

  final String panel;
  final Offset pivot;
  final Offset grab;
  final Offset? corner;
  double angle = 0;
  Offset factor = const Offset(1, 1);

  /// The pattern refused a step: the rest of the drag does nothing.
  bool refused = false;
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
    ('left_leg', l.aroundLeftLeg),
    ('right_leg', l.aroundRightLeg),
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
        // A cut piece keeps the whole's reference.
        'centre_x_mm': current['centre_x_mm'],
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
          label: around.endsWith('_arm') ? l.aroundArmAngle : l.aroundAngle,
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
    this.signed = false,
    this.formula = false,
    this.autofocus = false,
    this.hint,
  });

  /// Shown while the field is empty (the value used then).
  final String? hint;

  final TextEditingController controller;
  final FocusNode? focus;
  final String unit;
  final bool enabled;
  final VoidCallback? onSubmitted;
  final bool autofocus;

  /// Negative numbers may be typed.
  final bool signed;

  /// A formula may be typed instead of a number (`waist / 4 + 10 mm`).
  final bool formula;

  /// Typed characters and Enter go to the field, not to the window's
  /// one-key shortcuts (1–4 would turn the camera, R add a piece, Enter
  /// finish a drawing instead of submitting the field).
  static KeyEventResult _keepTyping(FocusNode _, KeyEvent event) {
    final keys = HardwareKeyboard.instance;
    final enter =
        event.logicalKey == LogicalKeyboardKey.enter ||
        event.logicalKey == LogicalKeyboardKey.numpadEnter;
    return (enter || event.character != null) &&
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
        autofocus: autofocus,
        textAlign: TextAlign.right,
        style: ts(fontSize: 13, color: p.text, tabular: true),
        keyboardType: formula
            ? TextInputType.text
            : TextInputType.numberWithOptions(decimal: true, signed: signed),
        inputFormatters: [
          if (!formula)
            FilteringTextInputFormatter.allow(
              RegExp(signed ? r'[-0-9.,]' : r'[0-9.,]'),
            ),
        ],
        onSubmitted: (_) => onSubmitted?.call(),
        decoration: InputDecoration(
          isDense: true,
          hintText: hint,
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

/// The piece's name, big; the pencil or a double click turns it into a
/// field. Enter, the tick, a click elsewhere or leaving the field renames
/// the piece; Esc keeps the name.
class _PieceName extends StatefulWidget {
  const _PieceName({super.key, required this.name, required this.onRename});

  final String name;
  final ValueChanged<String> onRename;

  @override
  State<_PieceName> createState() => _PieceNameState();
}

class _PieceNameState extends State<_PieceName> {
  final _text = TextEditingController();
  final _focus = FocusNode();
  var _editing = false;

  @override
  void initState() {
    super.initState();
    _focus.addListener(() {
      if (!_focus.hasFocus && _editing) _commit();
    });
  }

  @override
  void dispose() {
    _text.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _edit() {
    _text.value = TextEditingValue(
      text: widget.name,
      selection: TextSelection(baseOffset: 0, extentOffset: widget.name.length),
    );
    setState(() => _editing = true);
  }

  void _commit() {
    if (!_editing) return;
    final name = _text.text.trim();
    setState(() => _editing = false);
    if (name.isNotEmpty && name != widget.name) widget.onRename(name);
  }

  KeyEventResult _key(FocusNode node, KeyEvent event) {
    if (event is KeyDownEvent &&
        event.logicalKey == LogicalKeyboardKey.escape) {
      setState(() => _editing = false);
      return KeyEventResult.handled;
    }
    return _NumberField._keepTyping(node, event);
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final style = ts(fontSize: 24, fontWeight: FontWeight.w600, color: p.text);
    if (_editing) {
      return Focus(
        canRequestFocus: false,
        skipTraversal: true,
        onKeyEvent: _key,
        child: TextField(
          key: const Key('piece-name-field'),
          controller: _text,
          focusNode: _focus,
          autofocus: true,
          style: style,
          onSubmitted: (_) => _commit(),
          // A click anywhere else renames too, as leaving the field does.
          onTapOutside: (_) => _commit(),
          decoration: InputDecoration(
            isDense: true,
            contentPadding: const EdgeInsets.symmetric(
              horizontal: 8,
              vertical: 6,
            ),
            border: const OutlineInputBorder(),
            suffixIcon: _IconAction(
              key: const Key('piece-name-ok'),
              icon: Icons.check_rounded,
              tip: context.l.renamePieceOk,
              onTap: _commit,
            ),
          ),
        ),
      );
    }
    return Row(
      children: [
        Flexible(
          child: GestureDetector(
            onDoubleTap: _edit,
            child: Text(
              widget.name,
              style: style,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ),
        _IconAction(
          key: const Key('piece-rename'),
          icon: Icons.edit_outlined,
          tip: context.l.renamePiece,
          onTap: _edit,
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

/// A rounded, translucent holder for buttons over a view.
class _Pill extends StatelessWidget {
  const _Pill({super.key, required this.child});

  final Widget child;

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
      child: child,
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

/// Simulate / pause, restart and the avatar's motion, over the 3D view.
class _SimControls extends StatelessWidget {
  const _SimControls({
    super.key,
    required this.status,
    required this.onRun,
    required this.onRestart,
    required this.onMotion,
    required this.captured,
    required this.library,
    required this.onLibraryMotion,
    required this.onImportMotion,
    required this.onMotionLibrary,
  });

  final SimStatus? status;
  final ValueChanged<bool> onRun;
  final VoidCallback onRestart;

  /// A motion's name, or `null` to stand still.
  final ValueChanged<String?> onMotion;

  /// The project's captured motions (their paths in the project), read
  /// when the menu opens.
  final List<String> Function() captured;

  /// The user's library's motions, by name, read when the menu opens.
  final List<String> Function() library;
  final ValueChanged<String> onLibraryMotion;
  final VoidCallback onImportMotion;
  final VoidCallback onMotionLibrary;

  /// The menu's values for importing a motion and for the library (never
  /// a motion's path), and the prefix of a library motion's value.
  static const _import = '*import';
  static const _manage = '*library';
  static const _fromLibrary = '*library:';

  /// A captured motion's label: its file name without `.bvh`.
  static String _label(String path) {
    final name = path.split('/').last;
    return name.toLowerCase().endsWith('.bvh')
        ? name.substring(0, name.length - 4)
        : name;
  }

  static List<(String?, String)> _motions(L l) => [
    (null, l.motionStill),
    ('walk', l.motionWalk),
    ('idle', l.motionIdle),
    ('turn', l.motionTurn),
    ('raise_arms', l.motionRaiseArms),
  ];

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
          _motionMenu(context),
        ],
      ),
    );
  }

  Widget _motionMenu(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final current = status?.motion;
    // Menu values cannot be null: '' stands for standing still.
    return PopupMenuButton<String>(
      key: const ValueKey('motionButton'),
      tooltip: l.motionTip,
      onSelected: (choice) {
        if (choice == _import) {
          onImportMotion();
        } else if (choice == _manage) {
          onMotionLibrary();
        } else if (choice.startsWith(_fromLibrary)) {
          onLibraryMotion(choice.substring(_fromLibrary.length));
        } else {
          onMotion(choice.isEmpty ? null : choice);
        }
      },
      itemBuilder: (_) {
        final files = captured();
        // Library motions the project already has a copy of are listed
        // once, as the project's.
        final names = {for (final f in files) _label(f)};
        final library = this.library().where((n) => !names.contains(n));
        return [
          for (final (motion, label) in _motions(l))
            CheckedPopupMenuItem(
              value: motion ?? '',
              checked: motion == current,
              child: Text(label),
            ),
          if (files.isNotEmpty || library.isNotEmpty) const PopupMenuDivider(),
          for (final path in files)
            CheckedPopupMenuItem(
              value: path,
              checked: path == current,
              child: Text(_label(path)),
            ),
          for (final name in library)
            CheckedPopupMenuItem(
              value: '$_fromLibrary$name',
              checked: false,
              child: Text(name),
            ),
          const PopupMenuDivider(),
          PopupMenuItem(
            key: const ValueKey('motionImport'),
            value: _import,
            child: Text(l.motionImport),
          ),
          PopupMenuItem(
            key: const ValueKey('motionLibrary'),
            value: _manage,
            child: Text(l.motionLibraryMenu),
          ),
        ];
      },
      icon: Icon(
        Icons.directions_walk_rounded,
        size: 19,
        color: current == null ? p.textMuted : p.accent,
      ),
    );
  }
}

/// The project's garments from the body out: each one's fabric, its
/// layer, and a way to delete it; and ways to add one, blank or from the
/// pattern library.
class _GarmentList extends StatelessWidget {
  const _GarmentList({
    required this.engine,
    required this.garments,
    required this.indicators,
    required this.run,
    required this.onLibrary,
  });

  final Engine engine;

  /// Opens the pattern library.
  final VoidCallback onLibrary;

  /// As `project.summary` lists them, next to the body first.
  final List<dynamic> garments;

  /// `production.indicators`' garments.
  final List<Map<String, dynamic>> indicators;
  final ValueChanged<Map<String, Object?>> run;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final several = garments.length > 1;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          l.garments,
          style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
        ),
        if (several) ...[
          const SizedBox(height: Space.xs),
          Text(
            l.garmentsHint,
            style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
          ),
        ],
        for (final (i, g) in garments.indexed) ...[
          const SizedBox(height: Space.m),
          _GarmentCard(
            engine: engine,
            garment: g as Map<String, dynamic>,
            figures: indicators.where((f) => f['id'] == g['id']).firstOrNull,
            layer: i,
            count: garments.length,
            run: run,
          ),
        ],
        const SizedBox(height: Space.m),
        OutlinedButton.icon(
          onPressed: () => run({
            'type': 'garment.add',
            'name': l.garmentNewName(garments.length + 1),
          }),
          icon: const Icon(Icons.add_rounded, size: 18),
          label: Text(l.addGarment),
        ),
        const SizedBox(height: Space.s),
        OutlinedButton.icon(
          key: const ValueKey('openPatternLibrary'),
          onPressed: onLibrary,
          icon: const Icon(Icons.collections_bookmark_outlined, size: 18),
          label: Text(l.libraryOpen),
        ),
      ],
    );
  }
}

/// One garment: its name and layer, moved under or over its neighbours,
/// taken out of the drape or put back, deleted, and its fabric.
class _GarmentCard extends StatelessWidget {
  const _GarmentCard({
    required this.engine,
    required this.garment,
    required this.figures,
    required this.layer,
    required this.count,
    required this.run,
  });

  final Engine engine;
  final Map<String, dynamic> garment;

  /// Its `production.indicators` entry: fabric details and totals.
  final Map<String, dynamic>? figures;

  /// Its place in the list, 0 next to the body.
  final int layer;
  final int count;
  final ValueChanged<Map<String, Object?>> run;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final id = garment['id'] as String;
    final draped = garment['draped'] as bool? ?? true;
    void setLayer(int to) =>
        run({'type': 'garment.set_layer', 'garment': id, 'layer': to});
    return Container(
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        border: Border.all(color: p.border),
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // The name and the layer on their own lines: beside the
          // buttons, a narrow inspector left them a letter per line.
          Text(
            garment['name'] as String,
            style: ts(
              fontSize: 13.5,
              fontWeight: FontWeight.w600,
              color: p.text,
            ),
            overflow: TextOverflow.ellipsis,
          ),
          if (count > 1)
            Text(
              !draped
                  ? l.garmentOutOfDrape
                  : layer == 0
                  ? l.garmentNextToBody
                  : l.garmentOver,
              style: ts(fontSize: 12, color: p.textMuted),
            ),
          Wrap(
            children: [
              _IconAction(
                key: ValueKey('saveToLibrary:$id'),
                icon: Icons.bookmark_add_outlined,
                tip: l.librarySave,
                onTap: () async {
                  final said = await saveToLibrary(context, engine, garment);
                  if (said == null || !context.mounted) return;
                  ScaffoldMessenger.of(context)
                    ..hideCurrentSnackBar()
                    ..showSnackBar(
                      SnackBar(
                        content: Text(said),
                        behavior: SnackBarBehavior.floating,
                      ),
                    );
                },
              ),
              if (count > 1) ...[
                _IconAction(
                  icon: Icons.arrow_upward_rounded,
                  tip: l.garmentMoveIn,
                  onTap: layer > 0 ? () => setLayer(layer - 1) : null,
                ),
                _IconAction(
                  icon: Icons.arrow_downward_rounded,
                  tip: l.garmentMoveOut,
                  onTap: layer < count - 1 ? () => setLayer(layer + 1) : null,
                ),
                _IconAction(
                  icon: draped
                      ? Icons.layers_clear_outlined
                      : Icons.layers_outlined,
                  tip: draped ? l.garmentTakeOut : l.garmentPutBack,
                  onTap: () => run({
                    'type': 'garment.set_draped',
                    'garment': id,
                    'draped': !draped,
                  }),
                ),
                _IconAction(
                  icon: Icons.delete_outline_rounded,
                  tip: l.garmentRemove,
                  onTap: () => run({
                    'type': 'garment.remove',
                    'garment': id,
                    'confirm': true,
                  }),
                ),
              ],
            ],
          ),
          if (garment['source'] case final String file) ...[
            const SizedBox(height: Space.xs),
            Text(
              l.garmentSource(file.split('/').last),
              style: ts(fontSize: 12, color: p.textMuted),
            ),
            const SizedBox(height: Space.xs),
            Tooltip(
              message: l.garmentRedraftTip,
              child: OutlinedButton.icon(
                onPressed: () =>
                    run({'type': 'garment.redraft', 'garment': id}),
                icon: const Icon(Icons.autorenew_rounded, size: 18),
                label: Text(l.garmentRedraft),
              ),
            ),
          ],
          const SizedBox(height: Space.s),
          _FabricPicker(
            current: garment['fabric'] as String? ?? 'cotton',
            onPick: (fabric) => run({
              'type': 'garment.set_fabric',
              'garment': id,
              'fabric': fabric,
            }),
          ),
          const SizedBox(height: Space.m),
          _PrintPicker(engine: engine, garment: garment, run: run),
          if (figures case final f?) ...[
            const SizedBox(height: Space.m),
            _FabricDetails(garment: id, figures: f, run: run),
          ],
          const SizedBox(height: Space.m),
          _GarmentNotes(
            garment: id,
            notes: garment['notes'] as String? ?? '',
            run: run,
          ),
        ],
      ),
    );
  }
}

/// The garment's notes for the tech pack: free text, saved when the field
/// is left (one undo step per edit).
class _GarmentNotes extends StatefulWidget {
  const _GarmentNotes({
    required this.garment,
    required this.notes,
    required this.run,
  });

  final String garment;
  final String notes;
  final ValueChanged<Map<String, Object?>> run;

  @override
  State<_GarmentNotes> createState() => _GarmentNotesState();
}

class _GarmentNotesState extends State<_GarmentNotes> {
  late final _text = TextEditingController(text: widget.notes);
  final _focus = FocusNode();

  @override
  void initState() {
    super.initState();
    _focus.addListener(() {
      if (!_focus.hasFocus) _submit();
    });
  }

  @override
  void didUpdateWidget(_GarmentNotes old) {
    super.didUpdateWidget(old);
    if (!_focus.hasFocus && _text.text != widget.notes) {
      _text.text = widget.notes;
    }
  }

  @override
  void dispose() {
    _text.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _submit() {
    if (_text.text.trimRight() == widget.notes) return;
    widget.run({
      'type': 'garment.set_notes',
      'garment': widget.garment,
      'notes': _text.text,
    });
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l.garmentNotes,
          style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Focus(
          canRequestFocus: false,
          skipTraversal: true,
          onKeyEvent: _NumberField._keepTyping,
          child: TextField(
            key: const Key('garment-notes'),
            controller: _text,
            focusNode: _focus,
            minLines: 2,
            maxLines: 6,
            style: ts(fontSize: 13, color: p.text),
            decoration: InputDecoration(
              isDense: true,
              hintText: l.garmentNotesHint,
              hintStyle: ts(fontSize: 12, color: p.textMuted),
              contentPadding: const EdgeInsets.all(8),
              border: const OutlineInputBorder(),
            ),
          ),
        ),
      ],
    );
  }
}

/// The garment's fabric as bought (weight, width, price per metre), each
/// typed in a field, and what its pieces take of it.
class _FabricDetails extends StatefulWidget {
  const _FabricDetails({
    required this.garment,
    required this.figures,
    required this.run,
  });

  final String garment;

  /// Its `production.indicators` entry.
  final Map<String, dynamic> figures;
  final ValueChanged<Map<String, Object?>> run;

  @override
  State<_FabricDetails> createState() => _FabricDetailsState();
}

class _FabricDetailsState extends State<_FabricDetails> {
  final _weight = TextEditingController();
  final _width = TextEditingController();
  final _price = TextEditingController();
  final _focus = [FocusNode(), FocusNode(), FocusNode()];

  Map<String, dynamic> get _details =>
      (widget.figures['fabric_details'] as Map<String, dynamic>?) ?? const {};

  void _fill() {
    String shown(Object? v, double scale, int digits) =>
        v is num ? context.decimal(v / scale, digits) : '';
    _weight.text = shown(_details['weight_gsm'], 1, 0);
    _width.text = shown(_details['width_mm'], 10, 0);
    _price.text = shown(_details['price_per_m'], 1, 2);
  }

  @override
  void initState() {
    super.initState();
    for (final f in _focus) {
      f.addListener(() {
        if (!f.hasFocus) _submit();
      });
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _fill();
    });
  }

  @override
  void didUpdateWidget(_FabricDetails old) {
    super.didUpdateWidget(old);
    if (!_focus.any((f) => f.hasFocus)) _fill();
  }

  @override
  void dispose() {
    for (final c in [_weight, _width, _price]) {
      c.dispose();
    }
    for (final f in _focus) {
      f.dispose();
    }
    super.dispose();
  }

  void _submit() {
    final weight = _parseNumber(_weight.text);
    final width = _parseNumber(_width.text);
    final price = _parseNumber(_price.text);
    final step = <String, Object?>{
      'type': 'garment.set_fabric_details',
      'garment': widget.garment,
      'weight_gsm': ?weight,
      'width_mm': ?(width == null ? null : width * 10),
      'price_per_m': ?price,
    };
    final now = <String, Object?>{
      'weight_gsm': ?weight,
      'width_mm': ?(width == null ? null : width * 10),
      'price_per_m': ?price,
    };
    final was = {
      for (final k in ['weight_gsm', 'width_mm', 'price_per_m'])
        if (_details[k] case final num v) k: v.toDouble(),
    };
    if (now.length == was.length &&
        now.entries.every((e) => was[e.key] == e.value)) {
      return;
    }
    widget.run(step);
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final totals = widget.figures['totals'] as Map<String, dynamic>?;
    final usual = widget.figures['weight_gsm'] as num?;
    Widget field(
      String label,
      TextEditingController c,
      FocusNode f,
      String unit,
      String key, {
      String? hint,
    }) => Expanded(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: ts(fontSize: 11.5, color: p.textMuted)),
          const SizedBox(height: 2),
          _NumberField(
            key: Key(key),
            controller: c,
            focus: f,
            unit: unit,
            hint: hint,
            onSubmitted: _submit,
          ),
        ],
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l.fabricBought,
          style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Row(
          children: [
            field(
              l.fabricWeight,
              _weight,
              _focus[0],
              'g/m²',
              'fabric-weight',
              hint: usual == null ? null : context.decimal(usual, 0),
            ),
            const SizedBox(width: Space.xs),
            field(l.fabricWidth, _width, _focus[1], l.cm, 'fabric-width'),
            const SizedBox(width: Space.xs),
            field(l.fabricPrice, _price, _focus[2], '', 'fabric-price'),
          ],
        ),
        if (totals != null) ...[
          const SizedBox(height: Space.s),
          Text(
            l.garmentTotals(
              totals['pieces'] as int,
              context.number((totals['cut_area_mm2'] as num) / 1e6, 2),
              context.number(totals['mass_g'] as num, 0),
            ),
            style: ts(fontSize: 12, height: 1.4, color: p.textMuted),
          ),
        ],
      ],
    );
  }
}

/// Previews of prints already drawn, by what they show (garment, print,
/// colour), so rebuilding the inspector does not draw them again.
final _printPreviews = <String, Uint8List?>{};

/// The garment's print (spec 25): none, a built-in drawn in its colour,
/// or an image the user adds; one repeat's size and the angle to the
/// grainline.
class _PrintPicker extends StatelessWidget {
  const _PrintPicker({
    required this.engine,
    required this.garment,
    required this.run,
  });

  final Engine engine;

  /// As `project.summary` lists it.
  final Map<String, dynamic> garment;
  final ValueChanged<Map<String, Object?>> run;

  static const builtins = ['stripes', 'gingham', 'dots', 'twill'];

  static String _name(L l, String id) => switch (id) {
    'stripes' => l.printStripes,
    'gingham' => l.printGingham,
    'dots' => l.printDots,
    'twill' => l.printTwill,
    _ => id,
  };

  String get _id => garment['id'] as String;
  Map<String, dynamic>? get _print => garment['print'] as Map<String, dynamic>?;

  /// One repeat of `builtin` in the garment's colour, or of its own
  /// print; null when there is none or it cannot be read.
  Uint8List? _preview(String? builtin) {
    final key = [
      _id,
      builtin ?? jsonEncode(_print?['source']),
      garment['color'],
      garment['fabric'],
    ].join('|');
    if (_printPreviews.length > 64) _printPreviews.clear();
    return _printPreviews.putIfAbsent(key, () {
      try {
        final png = engine.printPreview(garment: _id, builtin: builtin);
        return png.isEmpty ? null : png;
      } on ApiError {
        return null;
      }
    });
  }

  /// Set the print: `source` (or the current one), keeping what is not
  /// given.
  void _set({
    Map<String, Object?>? source,
    double? width,
    double? angle,
    bool keepSize = true,
  }) {
    final now = _print;
    run({
      'type': 'garment.set_print',
      'garment': _id,
      'print': source ?? now?['source'],
      if (width != null)
        'width_mm': width
      else if (keepSize && now?['width_mm'] != null)
        'width_mm': now!['width_mm'],
      if (keepSize && now?['height_mm'] != null && width == null)
        'height_mm': now!['height_mm'],
      'angle_deg': angle ?? (now?['angle_deg'] as num?)?.toDouble() ?? 0,
    });
  }

  Future<void> _addImage(BuildContext context) async {
    final l = context.l;
    final messenger = ScaffoldMessenger.of(context);
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(
          label: l.printImages,
          extensions: const ['png', 'jpg', 'jpeg'],
        ),
      ],
    );
    if (file == null || !context.mounted) return;
    try {
      final path = await engine.adoptTexture(path: file.path);
      _set(source: {'image': path}, keepSize: false);
    } on ApiError catch (e) {
      if (!context.mounted) return;
      messenger.showSnackBar(
        SnackBar(
          content: Text(context.error(e)),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final print = _print;
    final source = print?['source'] as Map<String, dynamic>?;
    final current = source?['builtin'] as String?;
    final image = source?['image'] as String?;
    Widget swatch({
      required String tip,
      required bool on,
      required VoidCallback onTap,
      required Widget child,
      Key? key,
    }) => Tooltip(
      message: tip,
      child: InkWell(
        key: key,
        onTap: onTap,
        customBorder: const CircleBorder(),
        child: AnimatedContainer(
          duration: Motion.fast,
          width: 34,
          height: 34,
          padding: const EdgeInsets.all(3),
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            border: Border.all(
              color: on ? p.accent : p.border,
              width: on ? 2 : 1,
            ),
          ),
          child: ClipOval(child: child),
        ),
      ),
    );
    final tile = print == null ? null : _preview(null);
    final width = (print?['width_mm'] as num?)?.toDouble() ?? 20;
    final angle = (print?['angle_deg'] as num?)?.toDouble() ?? 0;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l.printTitle,
          style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Text(l.printHint, style: ts(fontSize: 12, color: p.textMuted)),
        const SizedBox(height: Space.s),
        Wrap(
          spacing: Space.s,
          runSpacing: Space.s,
          children: [
            swatch(
              key: const ValueKey('print-none'),
              tip: l.printNone,
              on: print == null,
              onTap: () =>
                  run({'type': 'garment.remove_print', 'garment': _id}),
              child: Icon(Icons.block_rounded, size: 18, color: p.textMuted),
            ),
            for (final b in builtins)
              swatch(
                key: ValueKey('print-$b'),
                tip: _name(l, b),
                on: current == b,
                onTap: () => _set(source: {'builtin': b}, keepSize: false),
                child: switch (_preview(b)) {
                  final png? => Image.memory(
                    png,
                    fit: BoxFit.cover,
                    gaplessPlayback: true,
                  ),
                  null => const SizedBox.shrink(),
                },
              ),
            swatch(
              key: const ValueKey('print-image'),
              tip: l.printImage,
              on: image != null,
              onTap: () => _addImage(context),
              child: Icon(
                Icons.add_photo_alternate_outlined,
                size: 18,
                color: p.textMuted,
              ),
            ),
          ],
        ),
        if (print != null) ...[
          const SizedBox(height: Space.m),
          Row(
            children: [
              Container(
                width: 56,
                height: 56,
                decoration: BoxDecoration(
                  border: Border.all(color: p.border),
                  borderRadius: BorderRadius.circular(4),
                ),
                clipBehavior: Clip.antiAlias,
                child: tile == null
                    ? Icon(Icons.broken_image_outlined, color: p.textMuted)
                    : Image.memory(
                        tile,
                        fit: BoxFit.cover,
                        gaplessPlayback: true,
                      ),
              ),
              const SizedBox(width: Space.s),
              Expanded(
                child: Text(
                  tile == null
                      ? l.printMissing
                      : image?.split('/').last ?? _name(l, current ?? ''),
                  style: ts(fontSize: 12, color: p.textMuted),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.m),
          _SliderRow(
            label: l.printWidth,
            fieldKey: const ValueKey('print-width'),
            value: (width / 10).clamp(0.2, 50),
            min: 0.2,
            max: 50,
            digits: 1,
            typedMin: 0.1,
            typedMax: 500,
            unit: l.cm,
            enabled: true,
            onChanged: (v) => _set(width: v * 10),
          ),
          const SizedBox(height: Space.l),
          _SliderRow(
            label: l.printAngle,
            value: angle.clamp(-90, 90),
            min: -90,
            max: 90,
            typedMin: -180,
            typedMax: 180,
            unit: '°',
            enabled: true,
            onChanged: (v) => _set(angle: v.roundToDouble()),
          ),
        ],
      ],
    );
  }
}

/// How the selected piece shifts and turns its garment's print, e.g. to
/// centre a motif or match stripes across a seam.
class _PiecePrint extends StatelessWidget {
  const _PiecePrint({required this.panel, required this.run});

  /// As `panel.get` returns it.
  final Map<String, dynamic> panel;
  final ValueChanged<Map<String, Object?>> run;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final print = panel['print'] as Map<String, dynamic>? ?? const {};
    final offset = (print['offset_mm'] as List?) ?? const [0, 0];
    final x = (offset[0] as num).toDouble();
    final y = (offset[1] as num).toDouble();
    final angle = (print['angle_deg'] as num?)?.toDouble() ?? 0;
    void set({double? dx, double? dy, double? turn}) => run({
      'type': 'panel.set_print_placement',
      'panel': panel['id'],
      'offset_mm': [dx ?? x, dy ?? y],
      'angle_deg': turn ?? angle,
    });
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l.piecePrint,
          style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.s),
        _SliderRow(
          label: l.piecePrintAcross,
          value: (x / 10).clamp(-20, 20),
          min: -20,
          max: 20,
          digits: 1,
          typedMin: -500,
          typedMax: 500,
          unit: l.cm,
          enabled: true,
          onChanged: (v) => set(dx: v * 10),
        ),
        const SizedBox(height: Space.l),
        _SliderRow(
          label: l.piecePrintUp,
          value: (y / 10).clamp(-20, 20),
          min: -20,
          max: 20,
          digits: 1,
          typedMin: -500,
          typedMax: 500,
          unit: l.cm,
          enabled: true,
          onChanged: (v) => set(dy: v * 10),
        ),
        const SizedBox(height: Space.l),
        _SliderRow(
          label: l.piecePrintTurn,
          fieldKey: const ValueKey('piece-print-turn'),
          value: angle.clamp(-90, 90),
          min: -90,
          max: 90,
          typedMin: -180,
          typedMax: 180,
          unit: '°',
          enabled: true,
          onChanged: (v) => set(turn: v.roundToDouble()),
        ),
      ],
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
/// The buttons, buttonholes and zippers on a piece (spec 26): button or
/// unbutton them, open a zipper part or all the way, remove them. The
/// drape follows while it runs.
class _TrimsCard extends StatelessWidget {
  const _TrimsCard({
    required this.panel,
    required this.trims,
    required this.run,
  });

  final String panel;
  final Map<String, dynamic> trims;
  final ValueChanged<Map<String, Object?>> run;

  static List<Map<String, dynamic>> _list(Map<String, dynamic> t, String k) => [
    for (final x in (t[k] as List?) ?? const []) x as Map<String, dynamic>,
  ];

  static List<Map<String, dynamic>> _buttons(
    Map<String, dynamic> t,
    String panel,
  ) => [
    for (final b in _list(t, 'buttons'))
      if (b['panel'] == panel || b['hole_panel'] == panel) b,
  ];

  static List<Map<String, dynamic>> _zippers(
    Map<String, dynamic> t,
    String panel,
  ) => [
    for (final z in _list(t, 'zippers'))
      if (z['panel_a'] == panel || z['panel_b'] == panel) z,
  ];

  /// Whether `panel` has any trim.
  static bool on(Map<String, dynamic> t, String panel) =>
      _buttons(t, panel).isNotEmpty || _zippers(t, panel).isNotEmpty;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final buttons = _buttons(trims, panel);
    final zippers = _zippers(trims, panel);
    Widget remove(Map<String, Object?> step) => IconButton(
      tooltip: l.trimRemove,
      visualDensity: VisualDensity.compact,
      icon: const Icon(Icons.close_rounded, size: 16),
      onPressed: () => run(step),
    );
    final label = ts(fontSize: 13, fontWeight: FontWeight.w500, color: p.text);
    return Container(
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        border: Border.all(color: p.border),
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l.trimsTitle,
            style: ts(
              fontSize: 13.5,
              fontWeight: FontWeight.w600,
              color: p.text,
            ),
          ),
          const SizedBox(height: Space.xs),
          for (final b in buttons)
            Row(
              children: [
                Expanded(
                  child: Text(
                    b['panel'] == panel
                        ? l.trimButton(
                            context.number(b['diameter_mm'] as num, 0),
                          )
                        : l.trimButtonhole(
                            context.number(b['diameter_mm'] as num, 0),
                            '${b['panel_name'] ?? b['panel']}',
                          ),
                    style: label,
                  ),
                ),
                // Its name as a tip: written on every row, it left the
                // button's own label a few letters a line.
                Tooltip(
                  message: l.trimButtoned,
                  child: Switch(
                    key: ValueKey('buttoned-${b['id']}'),
                    value: b['closed'] == true,
                    onChanged: (on) => run({
                      'type': 'button.set_closed',
                      'buttons': [b['id']],
                      'closed': on,
                    }),
                  ),
                ),
                remove({'type': 'button.remove', 'button': b['id']}),
              ],
            ),
          if (buttons.length > 1)
            Wrap(
              children: [
                for (final (closed, text) in [
                  (true, l.buttonAll),
                  (false, l.unbuttonAll),
                ])
                  TextButton(
                    onPressed: () => run({
                      'type': 'button.set_closed',
                      'buttons': [for (final b in buttons) b['id']],
                      'closed': closed,
                    }),
                    child: Text(text),
                  ),
              ],
            ),
          for (final z in zippers) ...[
            Row(
              children: [
                Expanded(
                  child: Text(
                    l.trimZipper(
                      context.number((z['length_mm'] as num? ?? 0) / 10, 1),
                    ),
                    style: label,
                  ),
                ),
                remove({'type': 'zipper.remove', 'zipper': z['id']}),
              ],
            ),
            KeyedSubtree(
              key: ValueKey('zipper-${z['id']}'),
              child: _SliderRow(
                fieldKey: ValueKey('zipper-open-${z['id']}'),
                label: l.trimOpen,
                value: (z['open_mm'] as num).toDouble() / 10,
                min: 0,
                max: ((z['length_mm'] as num? ?? 0) / 10).toDouble(),
                unit: l.cm,
                digits: 1,
                enabled: true,
                onChanged: (cm) => run({
                  'type': 'zipper.set_open',
                  'zipper': z['id'],
                  'open_mm': cm * 10,
                }),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// The piece's construction details (spec 31): each elastic with its
/// relaxed length, to change or remove.
class _DetailsCard extends StatelessWidget {
  const _DetailsCard({
    required this.panel,
    required this.details,
    required this.run,
  });

  final String panel;
  final Map<String, dynamic> details;
  final ValueChanged<Map<String, Object?>> run;

  static List<Map<String, dynamic>> _of(
    Map<String, dynamic> d,
    String kind,
    String panel,
  ) => [
    for (final e in (d[kind] as List?) ?? const [])
      if (e is Map<String, dynamic> && e['panel'] == panel) e,
  ];

  static List<Map<String, dynamic>> _elastics(
    Map<String, dynamic> d,
    String panel,
  ) => _of(d, 'elastics', panel);

  /// Whether `panel` has any detail.
  static bool on(Map<String, dynamic> d, String panel) =>
      _elastics(d, panel).isNotEmpty ||
      _of(d, 'topstitches', panel).isNotEmpty ||
      _of(d, 'interfacings', panel).isNotEmpty ||
      _of(d, 'quiltings', panel).isNotEmpty;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final label = ts(fontSize: 13, fontWeight: FontWeight.w500, color: p.text);
    return Container(
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        border: Border.all(color: p.border),
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l.detailsTitle,
            style: ts(
              fontSize: 13.5,
              fontWeight: FontWeight.w600,
              color: p.text,
            ),
          ),
          const SizedBox(height: Space.xs),
          for (final e in _elastics(details, panel)) ...[
            Row(
              children: [
                Expanded(
                  child: Text(
                    l.detailElastic(
                      context.number((e['relaxed_mm'] as num? ?? 0) / 10, 1),
                      context.number(e['width_mm'] as num, 0),
                    ),
                    style: label,
                  ),
                ),
                IconButton(
                  tooltip: l.trimRemove,
                  visualDensity: VisualDensity.compact,
                  icon: const Icon(Icons.close_rounded, size: 16),
                  onPressed: () =>
                      run({'type': 'elastic.remove', 'elastic': e['id']}),
                ),
              ],
            ),
            KeyedSubtree(
              key: ValueKey('elastic-${e['id']}'),
              child: _SliderRow(
                fieldKey: ValueKey('elastic-ratio-${e['id']}'),
                label: l.elasticRatio,
                value: (e['ratio'] as num).toDouble() * 100,
                min: 20,
                max: 100,
                unit: '%',
                enabled: true,
                onChanged: (pc) => run({
                  'type': 'elastic.set',
                  'elastic': e['id'],
                  'ratio': pc / 100,
                }),
              ),
            ),
          ],
          for (final q in _of(details, 'quiltings', panel))
            Row(
              children: [
                Expanded(
                  child: Text(
                    l.detailQuilting(
                      q['pattern'] == 'channels'
                          ? l.quiltingChannels
                          : l.quiltingDiamonds,
                      context.number((q['spacing_mm'] as num) / 10, 1),
                      context.number(q['loft_mm'] as num, 0),
                    ),
                    style: label,
                  ),
                ),
                IconButton(
                  tooltip: l.trimRemove,
                  visualDensity: VisualDensity.compact,
                  icon: const Icon(Icons.close_rounded, size: 16),
                  onPressed: () =>
                      run({'type': 'quilting.remove', 'quilting': q['id']}),
                ),
              ],
            ),
          for (final i in _of(details, 'interfacings', panel))
            Row(
              children: [
                Expanded(
                  child: Text(
                    l.detailInterfacing(switch (i['weight']) {
                      'light' => l.interfacingLight,
                      'heavy' => l.interfacingHeavy,
                      _ => l.interfacingMedium,
                    }, context.number((i['area_mm2'] as num? ?? 0) / 100, 0)),
                    style: label,
                  ),
                ),
                IconButton(
                  tooltip: l.trimRemove,
                  visualDensity: VisualDensity.compact,
                  icon: const Icon(Icons.close_rounded, size: 16),
                  onPressed: () => run({
                    'type': 'interfacing.remove',
                    'interfacing': i['id'],
                  }),
                ),
              ],
            ),
          for (final t in _of(details, 'topstitches', panel))
            Row(
              children: [
                Container(
                  width: 12,
                  height: 12,
                  margin: const EdgeInsets.only(right: Space.s),
                  decoration: BoxDecoration(
                    color: Color(
                      0xFF000000 |
                          int.parse(
                            (t['color'] as String).substring(1),
                            radix: 16,
                          ),
                    ),
                    border: Border.all(color: p.border),
                    borderRadius: BorderRadius.circular(3),
                  ),
                ),
                Expanded(
                  child: Text(
                    l.detailTopstitch(
                      t['rows'] as int,
                      context.number(t['offset_mm'] as num, 1),
                      context.number(t['stitch_mm'] as num, 1),
                    ),
                    style: label,
                  ),
                ),
                IconButton(
                  tooltip: l.trimRemove,
                  visualDensity: VisualDensity.compact,
                  icon: const Icon(Icons.close_rounded, size: 16),
                  onPressed: () =>
                      run({'type': 'topstitch.remove', 'topstitch': t['id']}),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

class _EdgeCard extends StatelessWidget {
  const _EdgeCard({
    required this.edge,
    required this.seams,
    required this.run,
    required this.unfold,
    required this.onArc,
  });

  final Map<String, dynamic> edge;
  final List<dynamic> seams;
  final ValueChanged<Map<String, Object?>> run;

  /// Unfold the piece on this edge (null when it cannot be its axis).
  final VoidCallback? unfold;

  /// Make this edge an arc (null when it follows other geometry).
  final VoidCallback? onArc;

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
          Wrap(
            children: [
              if (edge['curve']?['kind'] == 'cubic')
                TextButton.icon(
                  onPressed: () => run({
                    'type': 'edge.set_curve',
                    'edge': id,
                    'curve': {'kind': 'line'},
                  }),
                  icon: const Icon(Icons.horizontal_rule_rounded, size: 16),
                  label: Text(l.straighten),
                ),
              if (onArc case final arc?)
                TextButton.icon(
                  onPressed: arc,
                  icon: const Icon(Icons.looks_rounded, size: 16),
                  label: Text(l.makeArc),
                ),
              if (unfold case final unfold?)
                TextButton.icon(
                  onPressed: unfold,
                  icon: const Icon(Icons.flip_rounded, size: 16),
                  label: Text(l.unfold),
                ),
            ],
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
            _SliderRow(
              label: l.ease,
              value: ((seam['ease_mm'] as num?) ?? 0).toDouble() / 10,
              min: -5,
              max: 5,
              unit: l.cm,
              digits: 1,
              enabled: true,
              onChanged: (cm) => run({
                'type': 'seam.set_ease',
                'seam': seam['id'],
                'mm': (cm * 100).roundToDouble() / 10,
              }),
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
  const _AgentButton({required this.status, required this.onOpen});

  final AgentStatus status;
  final VoidCallback onOpen;

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
    return Tooltip(
      message: l.agentsTip,
      child: InkWell(
        key: const ValueKey('agentsButton'),
        onTap: onOpen,
        borderRadius: BorderRadius.circular(999),
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
      ),
    );
  }
}

/// The corners picked on the board: with two, their distance, typed or
/// with a slider (the second one moves along the line).
class _PointsCard extends StatelessWidget {
  const _PointsCard({
    required this.points,
    required this.run,
    required this.grading,
    required this.panel,
  });

  final List<PickedPoint> points;
  final ValueChanged<Map<String, Object?>> run;

  /// The project's size range, to grade one point; null without one (or
  /// on a linked mirror copy, which follows its source).
  final Map<String, dynamic>? grading;

  /// The piece, as `panel.get` gives it (with its points' grades).
  final Map<String, dynamic> panel;

  /// The grade of point [id] (an offset per size), empty if none.
  List<Offset> _grade(String id) {
    for (final e in panel['edges'] as List) {
      final start = e['start'] as Map<String, dynamic>;
      if (start['id'] == id) {
        return [
          for (final g in (start['grade'] as List?) ?? const [])
            Offset((g[0] as num).toDouble(), (g[1] as num).toDouble()),
        ];
      }
    }
    return const [];
  }

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
          if ((points.length, grading) case (1, final g?)) ...[
            const SizedBox(height: Space.m),
            _GradeControl(
              key: ValueKey('grade-${points.first.id}'),
              grade: _grade(points.first.id),
              base: (g['sizes'] as List).indexOf(g['base']),
              onStep: (step) => run({
                'type': 'point.set_grade',
                'point': points.first.id,
                'step': [step.dx, step.dy],
              }),
            ),
          ],
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

/// The project's size range: the sizes, the base one the pattern is drawn
/// in, the one shown in 3D; a chart to set or change it.
class _SizeRange extends StatelessWidget {
  const _SizeRange({
    required this.grading,
    required this.run,
    required this.query,
    required this.showOnBoard,
    required this.onShowOnBoard,
  });

  /// As `project.summary` gives it: `sizes`, `base`, `shown`; or null.
  final Map<String, dynamic>? grading;
  final ValueChanged<Map<String, Object?>> run;
  final dynamic Function(Map<String, Object?>) query;

  /// Whether the board shows the other sizes' outlines around each piece.
  final bool showOnBoard;
  final ValueChanged<bool> onShowOnBoard;

  Future<void> _pick(BuildContext context) async {
    final charts = query({'type': 'grading.charts'}) as List<dynamic>?;
    if (charts == null) return;
    final step = await showDialog<Map<String, Object?>>(
      context: context,
      builder: (_) => _SizeChartDialog(charts: charts, current: grading),
    );
    if (step != null) run(step);
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final g = grading;
    final title = Text(
      l.sizes,
      style: ts(fontSize: 14, fontWeight: FontWeight.w600, color: p.text),
    );
    if (g == null) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          title,
          const SizedBox(height: Space.xs),
          Text(
            l.sizesHint,
            style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
          ),
          const SizedBox(height: Space.m),
          OutlinedButton.icon(
            onPressed: () => _pick(context),
            icon: const Icon(Icons.straighten_rounded, size: 18),
            label: Text(l.addSizes),
          ),
        ],
      );
    }
    final sizes = [for (final s in g['sizes'] as List) s as String];
    final base = g['base'] as String?;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        title,
        const SizedBox(height: Space.s),
        Wrap(
          spacing: Space.xs,
          runSpacing: Space.xs,
          children: [
            for (final s in sizes)
              Chip(
                label: Text(s == base ? '$s · ${l.sizeBaseTag}' : s),
                visualDensity: VisualDensity.compact,
                side: BorderSide(color: s == base ? p.accent : p.border),
              ),
          ],
        ),
        const SizedBox(height: Space.m),
        DropdownButtonFormField<String>(
          key: ValueKey('shown-${g['shown']}'),
          initialValue: g['shown'] as String?,
          decoration: InputDecoration(labelText: l.sizeShown),
          items: [
            for (final s in sizes) DropdownMenuItem(value: s, child: Text(s)),
          ],
          onChanged: (to) => run({'type': 'grading.show_size', 'size': to}),
        ),
        const SizedBox(height: Space.s),
        Row(
          children: [
            Expanded(
              child: Text(
                l.sizesOnBoard,
                style: ts(fontSize: 13, color: p.text),
              ),
            ),
            Switch(
              key: const Key('sizes-on-board'),
              value: showOnBoard,
              onChanged: onShowOnBoard,
            ),
          ],
        ),
        const SizedBox(height: Space.s),
        Row(
          children: [
            Expanded(
              child: OutlinedButton(
                onPressed: () => _pick(context),
                child: Text(l.sizesChange),
              ),
            ),
            const SizedBox(width: Space.s),
            _IconAction(
              icon: Icons.delete_outline_rounded,
              tip: l.sizesRemove,
              onTap: () => run({'type': 'grading.clear', 'confirm': true}),
            ),
          ],
        ),
      ],
    );
  }
}

/// A size range from a built-in chart: the chart, the smallest and
/// largest sizes and the base size. Pops a `grading.use_chart` step.
class _SizeChartDialog extends StatefulWidget {
  const _SizeChartDialog({required this.charts, required this.current});

  /// As `grading.charts` gives them.
  final List<dynamic> charts;

  /// The range in use (`project.summary`), to start from.
  final Map<String, dynamic>? current;

  @override
  State<_SizeChartDialog> createState() => _SizeChartDialogState();
}

class _SizeChartDialogState extends State<_SizeChartDialog> {
  late String _chart;
  late String _from;
  late String _to;
  late String _base;

  List<String> _names(String chart) => [
    for (final c in widget.charts)
      if (c['chart'] == chart)
        for (final s in c['sizes'] as List) s['name'] as String,
  ];

  @override
  void initState() {
    super.initState();
    final current = widget.current;
    final sizes = [
      for (final s in (current?['sizes'] as List?) ?? const []) s as String,
    ];
    final chart = widget.charts
        .map((c) => c['chart'] as String)
        .firstWhere(
          (c) => sizes.isNotEmpty && sizes.every(_names(c).contains),
          orElse: () => widget.charts.first['chart'] as String,
        );
    _choose(chart);
    if (sizes.isNotEmpty && _names(chart).contains(sizes.first)) {
      _from = sizes.first;
      _to = sizes.last;
      _base = current?['base'] as String? ?? _base;
    }
  }

  /// Switch to [chart], with a range around its middle.
  void _choose(String chart) {
    final names = _names(chart);
    _chart = chart;
    final mid = names.length ~/ 2;
    _from = names[(mid - 2).clamp(0, names.length - 1)];
    _to = names[(mid + 4).clamp(0, names.length - 1)];
    _base = names[mid];
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    final p = Tokens.of(context);
    final names = _names(_chart);
    final (lo, hi) = (names.indexOf(_from), names.indexOf(_to));
    final at = names.indexOf(_base);
    final valid = lo < hi && at >= lo && at <= hi;
    Widget pick(
      String label,
      String value,
      List<String> options,
      ValueChanged<String> onPick,
    ) => DropdownButtonFormField<String>(
      key: ValueKey('$label-$_chart-${options.length}'),
      initialValue: options.contains(value) ? value : null,
      decoration: InputDecoration(labelText: label),
      items: [
        for (final s in options) DropdownMenuItem(value: s, child: Text(s)),
      ],
      onChanged: (v) {
        if (v != null) setState(() => onPick(v));
      },
    );
    final between = lo <= hi && lo >= 0
        ? names.sublist(lo, hi + 1)
        : const <String>[];
    return AlertDialog(
      title: Text(l.sizes),
      content: SizedBox(
        width: 440,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(l.sizeChart, style: ts(fontSize: 12.5, color: p.textMuted)),
            const SizedBox(height: Space.xs),
            SegmentedButton<String>(
              showSelectedIcon: false,
              segments: [
                ButtonSegment(value: 'women', label: Text(l.chartWomen)),
                ButtonSegment(value: 'men', label: Text(l.chartMen)),
                ButtonSegment(value: 'girls', label: Text(l.chartGirls)),
                ButtonSegment(value: 'boys', label: Text(l.chartBoys)),
                ButtonSegment(value: 'babies', label: Text(l.chartBabies)),
              ],
              selected: {_chart},
              onSelectionChanged: (s) => setState(() => _choose(s.first)),
            ),
            const SizedBox(height: Space.m),
            Row(
              children: [
                Expanded(
                  child: pick(l.sizeFrom, _from, names, (v) => _from = v),
                ),
                const SizedBox(width: Space.s),
                Expanded(child: pick(l.sizeTo, _to, names, (v) => _to = v)),
              ],
            ),
            const SizedBox(height: Space.m),
            pick(l.sizeBase, _base, between, (v) => _base = v),
            const SizedBox(height: Space.m),
            Text(
              l.sizesDialogHint,
              style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton(
          onPressed: valid
              ? () => Navigator.pop(context, <String, Object?>{
                  'type': 'grading.use_chart',
                  'chart': _chart,
                  'from': _from,
                  'to': _to,
                  'base': _base,
                })
              : null,
          child: Text(l.apply),
        ),
      ],
    );
  }
}

/// How one point moves from size to size: the same step (mm) from each
/// size to the next.
class _GradeControl extends StatefulWidget {
  const _GradeControl({
    super.key,
    required this.grade,
    required this.base,
    required this.onStep,
  });

  /// The point's offset in each size, empty if it is not graded.
  final List<Offset> grade;

  /// The base size's index.
  final int base;
  final ValueChanged<Offset> onStep;

  @override
  State<_GradeControl> createState() => _GradeControlState();
}

class _GradeControlState extends State<_GradeControl> {
  late final TextEditingController _x;
  late final TextEditingController _y;

  /// The step to the next size up (or from the one below, at the top).
  Offset get _step {
    final g = widget.grade;
    final b = widget.base;
    if (g.length < 2 || b < 0 || b >= g.length) return Offset.zero;
    return b + 1 < g.length ? g[b + 1] - g[b] : g[b] - g[b - 1];
  }

  /// Every size moves by the same step.
  bool get _uniform {
    final g = widget.grade;
    for (var i = 1; i < g.length; i++) {
      if ((g[i] - g[i - 1] - _step).distance > 0.05) return false;
    }
    return true;
  }

  static String _text(double v) {
    final r = (v * 10).round() / 10;
    return r == r.roundToDouble() ? r.toStringAsFixed(0) : r.toString();
  }

  @override
  void initState() {
    super.initState();
    _x = TextEditingController(text: _text(_step.dx));
    _y = TextEditingController(text: _text(_step.dy));
  }

  @override
  void dispose() {
    _x.dispose();
    _y.dispose();
    super.dispose();
  }

  void _submit() {
    final (x, y) = (_parseNumber(_x.text), _parseNumber(_y.text));
    if (x == null || y == null) return;
    widget.onStep(Offset(x, y));
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    Widget field(String axis, TextEditingController c) => Expanded(
      child: Row(
        children: [
          Text(axis, style: ts(fontSize: 12.5, color: p.textMuted)),
          const SizedBox(width: Space.xs),
          Expanded(
            child: _NumberField(
              key: Key('grade-$axis'),
              controller: c,
              unit: 'mm',
              signed: true,
              onSubmitted: _submit,
            ),
          ),
        ],
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          l.gradeStep,
          style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Text(
          _uniform ? l.gradeStepHint : l.gradeCustom,
          style: ts(fontSize: 12, height: 1.4, color: p.textMuted),
        ),
        const SizedBox(height: Space.s),
        Row(
          children: [
            field('x', _x),
            const SizedBox(width: Space.s),
            field('y', _y),
          ],
        ),
      ],
    );
  }
}
