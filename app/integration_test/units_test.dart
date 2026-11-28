// Every length in the user's units (docs/specs/38-region.md, M48 step 2):
// in inches, the inspector shows a piece's lengths in inches and a typed
// quarter inch is kept as such.

import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('a piece in inches', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    File('$dir/region.json')
        .writeAsStringSync('{"sizes": "us", "units": "imperial"}');
    final engine = Engine.open(
      path: await store.create(name: 'Inches', body: store.standardBody()),
    );
    dynamic run(Map<String, Object?> q) =>
        jsonDecode(engine.run(stepJson: jsonEncode(q)));
    final created =
        run({
              'type': 'panel.create_rect',
              'name': 'Square',
              'origin': [0, 0],
              'width': 200,
              'height': 400,
            })['created']
            as List;
    final piece = created.first as String;
    engine.select(panels: [piece]);

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

    // The sewing line, 120 cm, reads 47.24 in.
    expect(find.textContaining(RegExp(r'^47[.,]24 ')), findsOneWidget);

    // The seam allowance, 10 mm, reads 0.39 in; a quarter inch typed is
    // 6.35 mm, not rounded to a millimetre.
    final allowance = find.descendant(
      of: find.byKey(const Key('seam-allowance')),
      matching: find.byType(TextField),
    );
    await tester.ensureVisible(allowance);
    await frames();
    expect(
      tester.widget<TextField>(allowance).controller!.text,
      matches(RegExp(r'^0[.,]39$')),
    );
    await tester.enterText(allowance, '0.25');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await frames();
    final panel = run({'type': 'panel.get', 'panel': piece}) as Map;
    expect(panel['seam_allowance_mm'], closeTo(6.35, 1e-9));
  });
}
