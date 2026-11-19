// Prints (spec 25): the garment's card offers the built-in prints and an
// image; picking stripes puts them on the garment, the repeat can be
// typed, and None takes them off.

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

  testWidgets('stripes on the shirt, their repeat, then none', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(name: 'Shirt', body: store.standardBody()),
    );
    engine.run(
      stepJson: jsonEncode({'type': 'template.apply', 'template': 'shirt'}),
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

    Map<String, dynamic> garment() {
      final summary = jsonDecode(
        engine.run(stepJson: '{"type": "project.summary"}'),
      ) as Map<String, dynamic>;
      return (summary['garments'] as List).first as Map<String, dynamic>;
    }

    await idle();

    // The built-ins are drawn in the garment's colour for the swatches.
    final id = garment()['id'] as String;
    expect(engine.printPreview(garment: id, builtin: 'stripes'), isNotEmpty);
    expect(engine.printPreview(garment: id), isEmpty);

    // Stripes from the garment's card.
    final stripes = find.byKey(const ValueKey('print-stripes'));
    await tester.ensureVisible(stripes);
    await frames();
    await tester.tap(stripes);
    await frames();
    var print = garment()['print'] as Map<String, dynamic>;
    expect(print['source'], {'builtin': 'stripes'});
    expect(print['width_mm'], 20.0);
    expect(engine.printPreview(garment: id), isNotEmpty);

    // A wider repeat, typed in centimetres.
    final width = find.byKey(const ValueKey('print-width'));
    await tester.ensureVisible(width);
    await frames();
    await tester.enterText(width, '3');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await frames();
    print = garment()['print'] as Map<String, dynamic>;
    expect(print['width_mm'], 30.0);
    expect(print['source'], {'builtin': 'stripes'});

    // No print again.
    final none = find.byKey(const ValueKey('print-none'));
    await tester.ensureVisible(none);
    await frames();
    await tester.tap(none);
    await frames();
    expect(garment()['print'], isNull);
  });
}
