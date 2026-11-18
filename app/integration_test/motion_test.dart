// The avatar's motion (spec 22): the Motion menu next to Simulate makes
// the avatar walk while the garment drapes, ticks what it does, and makes
// it stand still again.

import 'dart:convert';
import 'dart:io';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('the avatar walks, then stands still', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(name: 'Skirt', body: store.standardBody()),
    );
    engine.run(
      stepJson: jsonEncode({
        'type': 'template.apply',
        'template': 'a_line_skirt',
      }),
    );

    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    Future<void> frames([int n = 12]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 40));
      }
    }

    Future<void> idle() async {
      for (var i = 0; i < 600; i++) {
        await frames(1);
        if (find.byType(ModalBarrier).evaluate().isEmpty) break;
      }
      await frames(5);
    }

    await idle();
    expect(engine.simulationStatus().motion, isNull);

    Finder item(String label) => find.ancestor(
      of: find.text(label),
      matching: find.byType(CheckedPopupMenuItem<String>),
    );
    Future<void> choose(String label) async {
      await tester.tap(find.byKey(const ValueKey('motionButton')));
      await frames();
      await tester.tap(item(label));
      await frames();
      await idle();
    }

    await choose('Walk');
    var status = engine.simulationStatus();
    expect(status.motion, 'walk');
    expect(status.running, isTrue);

    // The drape runs and the avatar moves in the 3D view.
    for (var i = 0; i < 1500 && engine.simulationStatus().timeS < 2.0; i++) {
      await frames(1);
    }
    expect(engine.simulationStatus().timeS, greaterThan(2.0));

    // The menu ticks the motion chosen.
    await tester.tap(find.byKey(const ValueKey('motionButton')));
    await frames();
    final ticked = find.byWidgetPredicate(
      (w) => w is CheckedPopupMenuItem<String> && w.checked,
    );
    expect(ticked, findsOneWidget);
    expect(
      (tester.widget(ticked) as CheckedPopupMenuItem<String>).value,
      'walk',
    );
    await tester.tap(item('Stand still'));
    await frames();
    await idle();
    expect(engine.simulationStatus().motion, isNull);
  });
}
