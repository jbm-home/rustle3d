// The Dimension tool on the board (spec 19): an edge's length placed and
// typed where it was placed; a height pulled out beside two corners, given
// a formula of the body's measurements.

import 'dart:convert';
import 'dart:io';
import 'dart:math' show max;

import 'package:flutter/services.dart';
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

  testWidgets('length typed in place, height from a formula', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(name: 'Panel', body: store.standardBody()),
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

    Future<void> click(num x, num y) async {
      await tester.tapAt(onBoard(x, y));
      await frames(3);
    }

    Map edgeAt(num x, num y) =>
        (run({'type': 'panel.get', 'panel': piece})['edges'] as List)
            .cast<Map>()
            .firstWhere((e) {
              final a = e['start']['pos'] as List;
              final b = e['end']['pos'] as List;
              final mid = Offset(
                ((a[0] as num) + (b[0] as num)) / 2,
                ((a[1] as num) + (b[1] as num)) / 2,
              );
              return (mid - Offset(x.toDouble(), y.toDouble())).distance < 1;
            });
    List<Map> constraints() =>
        (run({'type': 'constraint.list', 'panel': piece})['constraints']
                as List)
            .cast<Map>();
    final typed = find.byWidgetPredicate((w) => w is TextField && w.autofocus);

    await tester.sendKeyEvent(LogicalKeyboardKey.keyT);
    await frames(2);

    // The top edge (y = 0), placed above it, typed 25 cm.
    final top = edgeAt(150, 0);
    await click(150, 0);
    await click(150, -40);
    expect(constraints().single['kind'], 'length');
    expect(typed, findsOneWidget);
    await tester.enterText(typed, '25');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await frames();
    expect(typed, findsNothing);
    expect(constraints().single['mm'], closeTo(250, 0.01));
    final topNow = run({
      'type': 'panel.get',
      'panel': piece,
    })['edges'].firstWhere((e) => e['id'] == top['id']);
    expect(topNow['length_mm'], closeTo(250, 0.5));

    // The left side's two corners, pulled out beside them (to the right,
    // on the board): a height,
    // given a formula.
    // The edge furthest left (the solver moved the corners a little).
    num midX(Map e) =>
        ((e['start']['pos'][0] as num) + (e['end']['pos'][0] as num)) / 2;
    final left =
        ((run({'type': 'panel.get', 'panel': piece})['edges'] as List)
                .cast<Map>()
                .toList()
              ..sort((p, q) => midX(p).compareTo(midX(q))))
            .first;
    final a = left['start']['pos'] as List;
    final b = left['end']['pos'] as List;
    await click(a[0], a[1]);
    await click(b[0], b[1]);
    await click(
      max(a[0] as num, b[0] as num) + 80,
      ((a[1] as num) + (b[1] as num)) / 2,
    );
    final height = constraints().firstWhere((c) => c['kind'] == 'distance');
    expect(height['along'], 'vertical');
    await tester.enterText(typed, 'waist / 2');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await frames();
    final withFormula = constraints().firstWhere(
      (c) => c['id'] == height['id'],
    );
    expect(withFormula['formula'], 'waist / 2', reason: '$withFormula');
    expect(withFormula['holds'], true, reason: '$withFormula');
  });
}
