// The pattern edited on the board: dragging an edge reshapes its piece,
// as one undo step; dragging a piece moves it and its frame's handle
// turns it; a drag with the Draw tool puts a line on a piece; a trackpad
// pinch zooms it.

import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';
import 'package:rustle3d/src/views/rust_view.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('dragging an edge on the board reshapes the piece', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final path = await store.create(
      name: 'Skirt',
      body: store.standardBody(),
      template: 'a_line_skirt',
    );
    final engine = Engine.open(path: path);
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    String outlines() => jsonEncode(
      (jsonDecode(engine.run(stepJson: '{"type":"project.summary"}'))['panels']
              as List)
          .map((p) => p['area_mm2'])
          .toList(),
    );
    final before = outlines();
    final board = tester.getRect(
      find.byWidgetPredicate((w) => w is RustView && w.kind == ViewKind.board),
    );

    // Across the middle of the board a click lands on a side seam
    // somewhere (it selects the edge); drag it sideways from there.
    var at = Offset(board.left + 10, board.center.dy);
    while (engine.selectedEdge() == null && at.dx < board.right) {
      at += const Offset(2, 0);
      await tester.tapAt(at);
      await tester.pump();
    }
    expect(engine.selectedEdge(), isNotNull);
    final gesture = await tester.startGesture(
      at,
      kind: PointerDeviceKind.mouse,
    );
    for (var i = 0; i < 6; i++) {
      await gesture.moveBy(const Offset(5, 0));
      await tester.pump();
    }
    await gesture.up();
    await tester.pump();
    final changed = outlines() != before;
    expect(changed, isTrue);
    expect(engine.selection(), hasLength(1));

    engine.run(stepJson: '{"type":"history.undo"}');
    expect(outlines(), before);
  });

  testWidgets('dragging a piece moves it; its frame turns it', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final path = await store.create(
      name: 'Skirt',
      body: store.standardBody(),
      template: 'a_line_skirt',
    );
    final engine = Engine.open(path: path);
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    Map<String, dynamic> get(String id) => jsonDecode(
      engine.run(stepJson: jsonEncode({'type': 'panel.get', 'panel': id})),
    ) as Map<String, dynamic>;
    Rect box(String id) {
      final b = get(id)['bbox'];
      return Rect.fromPoints(
        Offset(
          (b['min'][0] as num).toDouble(),
          (b['min'][1] as num).toDouble(),
        ),
        Offset(
          (b['max'][0] as num).toDouble(),
          (b['max'][1] as num).toDouble(),
        ),
      );
    }

    final board = tester.getRect(
      find.byWidgetPredicate((w) => w is RustView && w.kind == ViewKind.board),
    );
    // Across the middle of the board, a click inside a piece, away from
    // its corners and edges, selects it and no edge.
    Offset? inside;
    for (
      var x = board.left + 10;
      x < board.right - 10 && inside == null;
      x += 6
    ) {
      engine.select(panels: const []);
      final at = Offset(x, board.center.dy);
      await tester.tapAt(at);
      await tester.pump();
      if (engine.selection().length == 1 && engine.selectedEdge() == null) {
        inside = at;
      }
    }
    final grab = inside!;
    final id = engine.selection().single;

    // How the board maps the screen to the pattern: the view under which
    // the point clicked is inside the piece.
    final ratio = RustView.pixelRatio(tester.view.devicePixelRatio);
    late Offset Function(Offset mm) toScreen;
    var mmPerPx = 0.0;
    for (var view = 0; view < 8 && mmPerPx == 0; view++) {
      List<double>? mmAt(Offset screen) {
        final px = (screen - board.topLeft) * ratio;
        return engine.toPattern(view: view, x: px.dx, y: px.dy);
      }

      final (a, b) = (mmAt(grab), mmAt(grab + const Offset(100, 0)));
      if (a == null || b == null || !box(id).contains(Offset(a[0], a[1]))) {
        continue;
      }
      mmPerPx = (b[0] - a[0]) / 100;
      toScreen = (mm) => grab + Offset(mm.dx - a[0], a[1] - mm.dy) / mmPerPx;
    }
    expect(mmPerPx, greaterThan(0));

    // Drag it: it moves, as one undo step.
    final before = box(id).center;
    final move = await tester.startGesture(grab, kind: PointerDeviceKind.mouse);
    for (var i = 0; i < 8; i++) {
      await move.moveBy(const Offset(5, 0));
      await tester.pump();
    }
    await move.up();
    await tester.pump();
    expect(box(id).center.dx - before.dx, closeTo(40 * mmPerPx, 2 * mmPerPx));
    expect(engine.selection(), [id]);
    engine.run(stepJson: '{"type":"history.undo"}');
    expect(box(id).center.dx, closeTo(before.dx, 1e-6));
    // (Undone behind the app's back: a click shows the piece as it is.)
    await tester.tapAt(grab);
    await tester.pump();

    // The round handle above the selected piece turns it: from above its
    // centre round to its right, a quarter turn clockwise.
    final c = box(id).center;
    final handle = Offset(c.dx, box(id).bottom + 40 * mmPerPx);
    final radius = handle.dy - c.dy;
    final turn = await tester.startGesture(
      toScreen(handle),
      kind: PointerDeviceKind.mouse,
    );
    for (var i = 1; i <= 9; i++) {
      final a = pi / 2 - i * pi / 18;
      await turn.moveTo(toScreen(c + Offset(cos(a) * radius, sin(a) * radius)));
      await tester.pump();
    }
    await turn.up();
    await tester.pump();
    final grain = (get(id)['grain_angle_deg'] as num).toDouble();
    expect(grain, closeTo(0, 1));
    engine.run(stepJson: '{"type":"history.undo"}');
    expect((get(id)['grain_angle_deg'] as num).toDouble(), closeTo(90, 1e-6));
  });

  testWidgets('a drag with the Draw tool puts a line on a piece', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final path = await store.create(
      name: 'Skirt',
      body: store.standardBody(),
      template: 'a_line_skirt',
    );
    final engine = Engine.open(path: path);
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    int lines() {
      final summary = jsonDecode(
        engine.run(stepJson: '{"type":"project.summary"}'),
      );
      return (summary['panels'] as List).fold<int>(0, (n, p) {
        final panel = jsonDecode(
          engine.run(
            stepJson: jsonEncode({'type': 'panel.get', 'panel': p['id']}),
          ),
        );
        return n + (panel['internal_lines'] as List).length;
      });
    }

    final board = tester.getRect(
      find.byWidgetPredicate((w) => w is RustView && w.kind == ViewKind.board),
    );
    final before = lines();
    await tester.sendKeyEvent(LogicalKeyboardKey.keyD);
    await tester.pump();
    // Across the board until a stroke starts on a piece.
    for (
      var x = board.left + 20;
      x < board.right - 20 && lines() == before;
      x += 20
    ) {
      final stroke = await tester.startGesture(
        Offset(x, board.center.dy - 30),
        kind: PointerDeviceKind.mouse,
      );
      for (var i = 0; i < 10; i++) {
        await stroke.moveBy(const Offset(0, 6));
        await tester.pump(const Duration(milliseconds: 16));
      }
      await stroke.up();
      await tester.pump();
    }
    // One line, or two when drawn on a piece with a linked mirror copy.
    expect(lines(), greaterThan(before));
    engine.run(stepJson: '{"type":"history.undo"}');
    expect(lines(), before);
  });

  testWidgets('a trackpad pinch zooms the board', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final path = await store.create(
      name: 'Skirt',
      body: store.standardBody(),
      template: 'a_line_skirt',
    );
    final engine = Engine.open(path: path);
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    final board = tester.getRect(
      find.byWidgetPredicate((w) => w is RustView && w.kind == ViewKind.board),
    );

    // How far from the board's centre, going right, the first edge is.
    Future<double> firstEdge() async {
      for (var dx = 2.0; dx < board.width / 2; dx += 2) {
        engine.select(panels: const []);
        await tester.tapAt(board.center + Offset(dx, 0));
        await tester.pump();
        if (engine.selectedEdge() != null) return dx;
      }
      return double.infinity;
    }

    final before = await firstEdge();
    final pinch = await tester.createGesture(kind: PointerDeviceKind.trackpad);
    await pinch.panZoomStart(board.center);
    for (var scale = 1.1; scale <= 1.6; scale += 0.1) {
      await pinch.panZoomUpdate(board.center, scale: scale);
      await tester.pump();
    }
    await pinch.panZoomEnd();
    await tester.pump();
    final after = await firstEdge();
    expect(after, greaterThan(before * 1.3), reason: '$before -> $after');
  });
}
