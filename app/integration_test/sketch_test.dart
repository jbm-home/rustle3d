// Sketching on the board (spec 19): a half piece drawn with Polygon and
// Half piece is unfolded; edges picked with a click and a ⇧-click get a
// constraint from the inspector's chips; a dimension typed in the
// Constraints section redraws the piece; a construction line is drawn
// with G; an ellipse is added from the Shapes dialog.

import 'dart:convert';
import 'dart:io';

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

  testWidgets('half piece, constraints, dimension, line, ellipse', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(name: 'Bodice', body: store.standardBody()),
    );
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    Future<void> frames([int n = 12]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 40));
      }
    }

    dynamic run(Map<String, Object?> q) =>
        jsonDecode(engine.run(stepJson: jsonEncode(q)));
    List<dynamic> panels() =>
        run({'type': 'project.summary'})['panels'] as List<dynamic>;

    // Opening fits the body (a busy card over the window): wait for it.
    for (var i = 0; i < 400; i++) {
      await frames(1);
      if (find.byType(ModalBarrier).evaluate().isEmpty) break;
    }
    await frames(5);
    final boardFinder = find.byWidgetPredicate(
      (w) => w is RustView && w.kind == ViewKind.board,
    );
    var board = tester.getRect(boardFinder);
    Offset at(double fx, double fy) =>
        board.topLeft + Offset(board.width * fx, board.height * fy);

    // A half piece: Polygon with Half piece on, four corners, Enter.
    await tester.sendKeyEvent(LogicalKeyboardKey.keyP);
    await frames(2);
    await tester.tap(find.widgetWithText(FilterChip, 'Half piece'));
    await frames(2);
    for (final f in const [
      Offset(0.5, 0.3),
      Offset(0.5, 0.7),
      Offset(0.7, 0.65),
      Offset(0.68, 0.32),
    ]) {
      await tester.tapAt(at(f.dx, f.dy));
      await frames(2);
    }
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await frames();
    final piece = panels().single;
    var got = run({'type': 'panel.get', 'panel': piece['id']}) as Map;
    expect(got['symmetry'], isNotNull);
    expect(got['point_count'], 6);

    // Click the two drawn horizontal-ish edges (the waist and the top),
    // picking the second with ⇧, then make them parallel.
    await tester.sendKeyEvent(LogicalKeyboardKey.keyV);
    await frames(2);
    final ratio = tester.view.devicePixelRatio;
    final drawn = [
      for (final e in got['edges'] as List)
        if (e['derived'] != true) e as Map,
    ];
    Future<void> clickEdge(Map e, {bool shift = false}) async {
      final a = e['start']['pos'] as List;
      final b = e['end']['pos'] as List;
      final mid = [
        ((a[0] as num) + (b[0] as num)) / 2,
        ((a[1] as num) + (b[1] as num)) / 2,
      ];
      for (var view = 0; view < 8; view++) {
        final px = engine.toView(
          view: view,
          xMm: mid[0].toDouble(),
          yMm: mid[1].toDouble(),
        );
        if (px == null) continue;
        final p = board.topLeft + Offset(px[0], px[1]) / ratio;
        if (!board.contains(p)) continue;
        if (shift) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await tester.tapAt(p);
        if (shift) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await frames(4);
        if (engine.selectedEdges().contains(e['id'])) return;
      }
    }

    await clickEdge(drawn[0]);
    await clickEdge(drawn[2], shift: true);
    expect(engine.selectedEdges(), [drawn[0]['id'], drawn[2]['id']]);
    await tester.tap(find.widgetWithText(ActionChip, 'Parallel'));
    await frames();
    var list = run({'type': 'constraint.list', 'panel': piece['id']}) as Map;
    expect((list['constraints'] as List).single['kind'], 'parallel');

    // One edge: keep its length, then type a new one in the Constraints
    // section: the piece is redrawn.
    await clickEdge(drawn[1]);
    await tester.tap(find.widgetWithText(ActionChip, 'Length'));
    await frames();
    list = run({'type': 'constraint.list', 'panel': piece['id']}) as Map;
    final length = (list['constraints'] as List).firstWhere(
      (c) => c['kind'] == 'length',
    );
    final field = find.byKey(ValueKey('${length['id']}-${length['mm']}'));
    await tester.scrollUntilVisible(
      field,
      100,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.enterText(
      find.descendant(of: field, matching: find.byType(TextField)),
      '12',
    );
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await frames();
    got = run({'type': 'panel.get', 'panel': piece['id']}) as Map;
    final edge = (got['edges'] as List).firstWhere(
      (e) => e['id'] == drawn[1]['id'],
    );
    expect(edge['length_mm'], closeTo(120, 0.01));

    // A construction line: G, two clicks (above the view's corner
    // buttons).
    board = tester.getRect(boardFinder);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyG);
    await frames(2);
    await tester.tapAt(at(0.1, 0.8));
    await frames(2);
    await tester.tapAt(at(0.7, 0.8));
    await frames();
    expect((run({'type': 'guide.list'}) as List).length, 1);

    // An ellipse: E, the dialog's defaults.
    await tester.sendKeyEvent(LogicalKeyboardKey.keyE);
    await frames();
    await tester.tap(find.widgetWithText(FilledButton, 'Create'));
    await frames();
    expect(panels().length, 2);
  });
}
