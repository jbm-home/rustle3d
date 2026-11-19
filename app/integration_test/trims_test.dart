// Trims (spec 26): the Button tool adds a button and its buttonhole with
// two clicks; the piece's card unbuttons it and opens a zipper.

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

  testWidgets('a button in two clicks, unbuttoned, a zipper opened', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(name: 'Fronts', body: store.standardBody()),
    );
    dynamic run(Map<String, Object?> q) =>
        jsonDecode(engine.run(stepJson: jsonEncode(q)));
    final left = run({
      'type': 'panel.create_rect',
      'name': 'Left',
      'origin': [0, 0],
      'width': 200,
      'height': 400,
    })['created'];
    final right = run({
      'type': 'panel.create_rect',
      'name': 'Right',
      'origin': [300, 0],
      'width': 200,
      'height': 400,
    })['created'];
    // A zipper between the left piece's right side and the right one's
    // left side, closed.
    run({
      'type': 'zipper.add',
      'a': {'edge': left[6]},
      'b': {'edge': right[8]},
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

    Map trims() => run({'type': 'trim.list'}) as Map;

    // The Button tool, from the rail's Trims menu: where the button goes,
    // then where it buttons.
    final trimsGroup = find.byIcon(Icons.radio_button_checked_rounded).first;
    await tester.ensureVisible(trimsGroup);
    await frames(3);
    await tester.tap(trimsGroup);
    await frames(5);
    await tester.tap(find.text('Button').last);
    await frames(5);
    await tester.tapAt(onBoard(150, 200));
    await frames(3);
    await tester.tapAt(onBoard(350, 200));
    await frames(5);
    final buttons = trims()['buttons'] as List;
    expect(buttons, hasLength(1));
    final button = buttons.single as Map;
    expect(button['panel'], left[0]);
    expect(button['hole_panel'], right[0]);
    expect(button['closed'], true);

    // The right piece is selected: its card unbuttons the button.
    final buttoned = find.byKey(ValueKey('buttoned-${button['id']}'));
    await tester.ensureVisible(buttoned);
    await frames();
    await tester.tap(buttoned);
    await frames();
    expect((trims()['buttons'] as List).single['closed'], false);

    // And opens the zipper 5 cm from the top.
    final zipper = (trims()['zippers'] as List).single as Map;
    final open = find.byKey(ValueKey('zipper-open-${zipper['id']}'));
    await tester.ensureVisible(open);
    await frames();
    await tester.enterText(open, '5');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await frames();
    expect((trims()['zippers'] as List).single['open_mm'], 50.0);
  });
}
