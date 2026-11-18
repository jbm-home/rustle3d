// The pattern edited on the board: dragging an edge reshapes its piece,
// as one undo step; a trackpad pinch zooms it.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/gestures.dart';
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
