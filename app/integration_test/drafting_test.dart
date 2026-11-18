// The drafting tools on the board: Polygon and Curve draw pieces, Notch
// adds a notch where an edge is clicked, Cut cuts a piece in two sewn
// pieces, and the Edit curves tools bend an edge, straighten it (from the
// group's menu) and add a point on it.

import 'dart:convert';
import 'dart:io';

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

  testWidgets('polygon, notch and curve on the board', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(
        name: 'Skirt',
        body: store.standardBody(),
        template: 'a_line_skirt',
      ),
    );
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    Future<void> frames([int n = 10]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 30));
      }
    }

    dynamic query(Map<String, Object?> q) =>
        jsonDecode(engine.run(stepJson: jsonEncode(q)));
    List<dynamic> panels() =>
        query({'type': 'project.summary'})['panels'] as List<dynamic>;
    int notches() => panels().fold(
      0,
      (n, p) =>
          n +
          (query({'type': 'panel.get', 'panel': p['id']})['notches'] as List)
              .length,
    );

    await frames(20);
    final board = tester.getRect(
      find.byWidgetPredicate((w) => w is RustView && w.kind == ViewKind.board),
    );

    // Polygon: three corners, then Enter.
    final pieces = panels().length;
    await tester.sendKeyEvent(LogicalKeyboardKey.keyP);
    await tester.pump();
    for (final f in const [
      Offset(0.1, 0.1),
      Offset(0.3, 0.1),
      Offset(0.2, 0.25),
    ]) {
      await tester.tapAt(
        board.topLeft + Offset(board.width * f.dx, board.height * f.dy),
      );
      await tester.pump();
    }
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await frames();
    expect(panels().length, pieces + 1);

    // Notch: across the middle, a click lands on an edge somewhere.
    final before = notches();
    await tester.sendKeyEvent(LogicalKeyboardKey.keyN);
    await tester.pump();
    for (
      var x = board.left + 10;
      x < board.right && notches() == before;
      x += 3
    ) {
      await tester.tapAt(Offset(x, board.center.dy));
      await tester.pump();
    }
    expect(notches(), before + 1);

    // Curve: a corner, a smooth point (dragged), a corner, then Enter.
    Offset at(double fx, double fy) =>
        board.topLeft + Offset(board.width * fx, board.height * fy);
    final before2 = panels().length;
    await tester.sendKeyEvent(LogicalKeyboardKey.keyC);
    await tester.pump();
    await tester.tapAt(at(0.7, 0.1));
    await tester.pump();
    await tester.dragFrom(at(0.85, 0.15), const Offset(20, 30));
    await tester.pump();
    await tester.tapAt(at(0.75, 0.3));
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await frames();
    expect(panels().length, before2 + 1);
    final drawn = query({'type': 'panel.get', 'panel': panels().last['id']});
    expect(
      (drawn['edges'] as List).where((e) => e['curve']['kind'] == 'cubic'),
      isNotEmpty,
    );

    // Cut: a line across the polygon drawn first, then Enter.
    final seams = (query({'type': 'seam.list'}) as List).length;
    await tester.sendKeyEvent(LogicalKeyboardKey.keyK);
    await tester.pump();
    await tester.tapAt(at(0.17, 0.08));
    await tester.pump();
    await tester.tapAt(at(0.17, 0.3));
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await frames();
    expect(panels().length, before2 + 2);
    expect((query({'type': 'seam.list'}) as List).length, seams + 1);

    // Bend: the first edge clicked gets handles (it becomes a cubic).
    await tester.sendKeyEvent(LogicalKeyboardKey.keyB);
    await tester.pump();
    engine.select(panels: const []);
    var spot = Offset.zero;
    for (
      var x = board.left + 10;
      x < board.right && engine.selectedEdge() == null;
      x += 3
    ) {
      spot = Offset(x, board.center.dy - 40);
      await tester.tapAt(spot);
      await tester.pump();
    }
    final edge = engine.selectedEdge()!;
    String kind() {
      final panel = query({
        'type': 'panel.get',
        'panel': engine.selection().first,
      });
      final e = (panel['edges'] as List).firstWhere((e) => e['id'] == edge);
      return e['curve']['kind'] as String;
    }

    expect(kind(), 'cubic');
    await frames();

    // Edit curves menu → Straighten, then a click on the curved edge.
    await tester.tap(find.bySemanticsLabel('Edit curves'));
    await frames();
    await tester.tap(find.text('Straighten').last);
    await frames();
    await tester.tapAt(spot);
    await frames();
    expect(kind(), 'line');

    // Add point (A): a click on an edge of the curved piece splits it
    // (the skirt's edges are sewn, and a sewn edge cannot be split).
    int edges() => panels().fold(
      0,
      (n, p) =>
          n +
          (query({'type': 'panel.get', 'panel': p['id']})['edges'] as List)
              .length,
    );
    final edgesBefore = edges();
    await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
    await tester.pump();
    for (var fx = 0.62; fx < 0.95 && edges() == edgesBefore; fx += 0.004) {
      await tester.tapAt(at(fx, 0.2));
      await tester.pump();
    }
    expect(edges(), edgesBefore + 1);
  });
}
