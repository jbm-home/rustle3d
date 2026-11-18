// Curve handles with Select (spec 19): a picked curved edge shows its two
// handles, a picked corner the handle of each curve meeting there; either
// is dragged, one undo step per drag.

import 'dart:convert';
import 'dart:io';

import 'package:material_ui/material_ui.dart';
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

  testWidgets('drag handles with Select', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(name: 'Curves', body: store.standardBody()),
    );
    dynamic run(Map<String, Object?> q) =>
        jsonDecode(engine.run(stepJson: jsonEncode(q)));
    final piece = run({
      'type': 'panel.create_rect',
      'name': 'Front',
      'origin': [0, 0],
      'width': 300,
      'height': 400,
    })['created'][0];
    Map edge(bool Function(Map e) test) =>
        (run({'type': 'panel.get', 'panel': piece})['edges'] as List)
            .cast<Map>()
            .firstWhere(test);
    // The top edge (y = 0), curved.
    bool isTop(Map e) => e['start']['pos'][1] == 0 && e['end']['pos'][1] == 0;
    final top = edge(isTop);
    final (sx, ex) = (
      top['start']['pos'][0] as num,
      top['end']['pos'][0] as num,
    );
    run({
      'type': 'edge.set_curve',
      'edge': top['id'],
      'curve': {
        'kind': 'cubic',
        'c1': [sx + (ex - sx) / 3, -30],
        'c2': [sx + (ex - sx) * 2 / 3, -30],
      },
    });

    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    Future<void> frames([int n = 12]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 40));
      }
    }

    for (var i = 0; i < 400; i++) {
      await frames(1);
      if (find.byType(ModalBarrier).evaluate().isEmpty) break;
    }
    await frames(5);
    final board = tester.getRect(
      find.byWidgetPredicate((w) => w is RustView && w.kind == ViewKind.board),
    );
    final ratio = tester.view.devicePixelRatio;
    Offset onBoard(num x, num y) {
      for (var view = 0; view < 8; view++) {
        final px = engine.toView(
          view: view,
          xMm: x.toDouble(),
          yMm: y.toDouble(),
        );
        if (px == null) continue;
        final p = board.topLeft + Offset(px[0], px[1]) / ratio;
        if (board.contains(p)) return p;
      }
      throw StateError('($x, $y) is not on the board');
    }

    Future<void> drag(List c, Offset by) async {
      final from = onBoard(c[0] as num, c[1] as num);
      final g = await tester.startGesture(from);
      await frames(1);
      for (var i = 1; i <= 6; i++) {
        await g.moveTo(from + by * (i / 6));
        await frames(1);
      }
      await g.up();
      await frames(3);
    }

    int steps() => (run({'type': 'history.list'})['steps'] as List).length;

    // Pick the curved edge on the curve's middle, then drag its first
    // handle down: one undo step.
    final curved = edge((e) => e['id'] == top['id']);
    await tester.tapAt(onBoard((sx + ex) / 2, -22.5));
    await frames(3);
    expect(engine.selectedEdges(), [top['id']]);
    final before = steps();
    final c1 = curved['curve']['c1'] as List;
    await drag(c1, const Offset(0, 30));
    final moved = edge((e) => e['id'] == top['id']);
    expect(moved['curve']['c1'], isNot(c1));
    expect(moved['curve']['c2'], curved['curve']['c2']);
    expect(steps(), before + 1);

    // Pick the edge's end corner: its handle there (c2) is dragged.
    final corner = moved['end']['pos'] as List;
    await tester.tapAt(onBoard(corner[0] as num, corner[1] as num));
    await frames(3);
    expect(engine.selectedPoints().map((p) => p.id), [moved['end']['id']]);
    final c2 = moved['curve']['c2'] as List;
    await drag(c2, const Offset(-25, 0));
    final again = edge((e) => e['id'] == top['id']);
    expect(again['curve']['c2'], isNot(c2));
    expect(again['end']['pos'], corner);
  });
}
