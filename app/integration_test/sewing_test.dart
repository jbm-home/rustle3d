// The Sew tool on the board (spec 21): a patch pocket's three edges,
// ⌘-clicked in any order, sewn to the U-shaped placement line drawn on the
// body piece; then a stretch dragged along the body's top edge sewn to the
// pocket's top edge.

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

  testWidgets('pocket on its line, part of an edge', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(name: 'Pocket', body: store.standardBody()),
    );
    dynamic run(Map<String, Object?> q) =>
        jsonDecode(engine.run(stepJson: jsonEncode(q)));
    final body = run({
      'type': 'panel.create_rect',
      'name': 'Body',
      'origin': [0, 0],
      'width': 300,
      'height': 400,
    })['created'][0];
    final pocket = run({
      'type': 'panel.create_rect',
      'name': 'Pocket',
      'origin': [400, 0],
      'width': 120,
      'height': 140,
    })['created'][0];
    final line = run({
      'type': 'internal_line.add',
      'panel': body,
      'start': [90, 100],
      'segments': [
        {
          'kind': 'line',
          'to': [90, 240],
        },
        {
          'kind': 'line',
          'to': [210, 240],
        },
        {
          'kind': 'line',
          'to': [210, 100],
        },
      ],
    })['created'][0];

    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    Future<void> frames([int n = 12]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 40));
      }
    }

    // Opening fits the body (a busy card over the window): wait for it.
    for (var i = 0; i < 400; i++) {
      await frames(1);
      if (find.byType(ModalBarrier).evaluate().isEmpty) break;
    }
    await frames(5);
    final board = tester.getRect(
      find.byWidgetPredicate((w) => w is RustView && w.kind == ViewKind.board),
    );
    final ratio = tester.view.devicePixelRatio;
    // The board's view id is not exposed: the one that places a pattern
    // point inside the board.
    Offset? onBoard(num x, num y) {
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
      return null;
    }

    List<Map> edgesOf(String panel) => [
      for (final e in run({'type': 'panel.get', 'panel': panel})['edges']
          as List)
        e as Map,
    ];
    (num, num) middle(Map e) {
      final a = e['start']['pos'] as List;
      final b = e['end']['pos'] as List;
      return (
        ((a[0] as num) + (b[0] as num)) / 2,
        ((a[1] as num) + (b[1] as num)) / 2,
      );
    }

    Future<void> click(num x, num y, {bool extend = false}) async {
      final p = onBoard(x, y)!;
      final ctrl = LogicalKeyboardKey.controlLeft;
      if (extend) await tester.sendKeyDownEvent(ctrl);
      await tester.tapAt(p);
      if (extend) await tester.sendKeyUpEvent(ctrl);
      await frames(3);
    }

    List<dynamic> seams() => run({'type': 'seam.list'}) as List<dynamic>;

    await tester.sendKeyEvent(LogicalKeyboardKey.keyS);
    await frames(2);

    // The pocket's sides and bottom (all but its top, at y = 140): two
    // ⌘-clicks out of order, a click on the last; then a click on the line.
    final pocketEdges = edgesOf(pocket);
    bool isTop(Map e) => middle(e).$2 == 140;
    final sides = pocketEdges.where((e) => !isTop(e)).toList();
    final top = pocketEdges.firstWhere(isTop);
    final (x2, y2) = middle(sides[2]);
    final (x0, y0) = middle(sides[0]);
    final (x1, y1) = middle(sides[1]);
    await click(x2, y2, extend: true);
    await click(x0, y0, extend: true);
    await click(x1, y1);
    await click(150, 240);
    final pocketSeam = seams().single as Map;
    expect(pocketSeam['b']['line'], line, reason: '$pocketSeam');
    expect(
      (pocketSeam['a']['edges'] as List).toSet(),
      {for (final e in sides) e['id']},
      reason: '$pocketSeam',
    );

    // Part of the body's top edge (y = 0), dragged from x = 60 to x = 180,
    // sewn to the pocket's top edge.
    final from = onBoard(60, 0)!;
    final to = onBoard(180, 0)!;
    final gesture = await tester.startGesture(from);
    await frames(1);
    for (var i = 1; i <= 8; i++) {
      await gesture.moveTo(Offset.lerp(from, to, i / 8)!);
      await frames(1);
    }
    await gesture.up();
    await frames(3);
    final (tx, ty) = middle(top);
    await click(tx, ty);
    final partial = seams().firstWhere((s) => s['id'] != pocketSeam['id']);
    final a = partial['a'] as Map;
    expect(a['from_mm'], isNotNull, reason: '$partial');
    final stretch = (a['to_mm'] as num) - (a['from_mm'] as num);
    expect(stretch, closeTo(120, 15), reason: '$partial');
    expect(partial['b']['edge'], top['id'], reason: '$partial');
  });
}
