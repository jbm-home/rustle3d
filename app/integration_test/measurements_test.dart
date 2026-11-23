// Body measurements (spec 29): the Body panel lists the other
// measurements, estimated; one typed is kept and the avatar follows it;
// the reset button brings the estimate back.

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

  testWidgets('type a neck, the avatar follows, back to the estimate', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final project = await store.create(
      name: 'Measured',
      body: store.standardBody(),
    );
    final engine = Engine.open(path: project);
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    Future<void> frames([int n = 12]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 40));
      }
    }

    // Wait out the busy card; the fit runs off the UI thread.
    Future<void> idle() async {
      for (var i = 0; i < 600; i++) {
        await tester.runAsync(
          () => Future.delayed(const Duration(milliseconds: 50)),
        );
        await frames(1);
        if (find.byType(ModalBarrier).evaluate().length <= 1) break;
      }
      await frames(5);
    }

    Map<String, dynamic> neck() {
      final got =
          jsonDecode(engine.run(stepJson: '{"type": "avatar.get"}'))
              as Map<String, dynamic>;
      return (got['measures'] as List)
          .cast<Map<String, dynamic>>()
          .firstWhere((m) => m['name'] == 'neck');
    }

    for (var i = 0; i < 600; i++) {
      await frames(1);
      if (find.byType(ModalBarrier).evaluate().isEmpty) break;
    }
    await frames(5);
    expect(neck()['estimated'], isTrue);

    await tester.tap(find.byTooltip('Body'));
    await frames();
    await tester.tap(find.text('Measurements…'));
    await frames();
    await tester.tap(find.byKey(const Key('moreMeasurements')));
    await frames();
    expect(find.byKey(const Key('measure_ankle')), findsOneWidget);

    final field = find.byKey(const Key('measure_neck'));
    await tester.ensureVisible(field);
    await frames();
    await tester.enterText(field, '40');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await idle();
    expect(neck()['estimated'], isFalse);
    expect(neck()['cm'], 40.0);
    expect((neck()['reached_cm'] as num).toDouble(), closeTo(40, 1));

    final reset = find.byKey(const Key('reset_neck'));
    expect(reset, findsOneWidget);
    await tester.tap(reset);
    await idle();
    expect(neck()['estimated'], isTrue);
    expect(find.byKey(const Key('reset_neck')), findsNothing);
  });
}
