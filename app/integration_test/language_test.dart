// The UI language: French when chosen (and remembered), pieces named in
// it, errors translated from their code; the hem changed from the
// inspector by a typed amount.

import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/i18n.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';
import 'package:rustle3d/src/views/rust_view.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('a French project, then back to English', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    LanguageMemory(dir).code = 'fr';
    File('$dir/tour-seen').createSync();
    final fr = await L.delegate.load(const Locale('fr'));

    // Template pieces get their French names, mirror copies included.
    final path = store.create(
      name: 'Jupe',
      body: store.standardBody(),
      template: 'a_line_skirt',
      pieceNames: pieceNames(fr),
    );
    final engine = Engine.open(path: path);
    final summary = jsonDecode(
      engine.run(stepJson: '{"type":"project.summary"}'),
    );
    final names = [for (final p in summary['panels'] as List) p['name']];
    expect(names, containsAll(['Devant', 'Dos gauche', 'Dos droit']));
    expect(engine.resetSimulation().loaded, isTrue);

    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    expect(find.text('Inspecteur'), findsOneWidget);

    // Pick the hem in 3D, then make the skirt 2 cm longer from the
    // inspector.
    final centre = tester.getCenter(
      find.byWidgetPredicate(
        (w) => w is RustView && w.kind == ViewKind.garment,
      ),
    );
    for (var dy = 60.0; dy < 300 && engine.selectedEdge() == null; dy += 2) {
      await tester.tapAt(centre + Offset(0, dy));
      await tester.pump();
    }
    expect(engine.selectedEdge(), isNotNull);
    await tester.pump();
    final before = jsonDecode(
      engine.run(stepJson: '{"type":"project.summary"}'),
    );
    await tester.enterText(find.widgetWithText(TextField, '1'), '2');
    await tester.tap(find.text('Rallonger'));
    await tester.pump();
    final after = jsonDecode(
      engine.run(stepJson: '{"type":"project.summary"}'),
    );
    double area(Map s) => (s['panels'] as List).fold(
      0.0,
      (sum, p) => sum + (p['area_mm2'] as num).toDouble(),
    );
    expect(area(after), greaterThan(area(before)));

    // An error from the core is shown in French.
    final context = tester.element(find.text('Rallonger'));
    expect(
      context.error(
        const ApiError(code: 'nothing_to_redo', args: {}, text: 'x'),
      ),
      'Rien à rétablir.',
    );

    // Switching to English takes effect at once and is remembered.
    Future<void> settle() async {
      for (var i = 0; i < 10; i++) {
        await tester.pump(const Duration(milliseconds: 50));
      }
    }

    await tester.tap(find.byType(LanguageButton));
    await settle();
    await tester.tap(find.text('English').last);
    await settle();
    expect(find.text('Rallonger'), findsNothing);
    expect(find.text('Longer'), findsOneWidget);
    expect(LanguageMemory(dir).code, 'en');
  });
}
