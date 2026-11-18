// Sizes end to end: a size range from a chart in the inspector, the size
// shown in 3D, a grade typed on one point, and the graded nest exported.

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

  testWidgets('a size range, the size shown, a graded point', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    // As the start screen makes it: the body, then the template with every
    // measurement of its form.
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    final engine = Engine.open(
      path: store.create(
        name: 'Skirt',
        body: store.standardBody(),
        template: 'straight_skirt',
        sizes: const {
          'waist_cm': 73,
          'hips_cm': 98,
          'length_cm': 60,
          'ease_cm': 2,
        },
      ),
    );
    dynamic run(Map<String, Object?> step) =>
        jsonDecode(engine.run(stepJson: jsonEncode(step)));
    // The views draw every frame: pump a while instead of settling.
    Future<void> settle() async {
      for (var i = 0; i < 12; i++) {
        await tester.pump(const Duration(milliseconds: 50));
      }
    }

    File('$dir/tour-seen').createSync();
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    await settle();

    // Nothing selected: the inspector offers a size range.
    final add = find.text('Add a size range');
    await tester.scrollUntilVisible(
      add,
      200,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.tap(add);
    await settle();
    expect(find.text('Women'), findsOneWidget);
    await tester.tap(find.text('Apply'));
    await settle();
    final grading = run({'type': 'grading.get'}) as Map<String, dynamic>;
    final sizes = [for (final s in grading['sizes'] as List) s['name']];
    expect(sizes.length, greaterThan(2));
    expect(sizes, contains(grading['base']));
    // The skirt, drafted before the sizes, is graded to every size.
    expect(grading['graded_points'] as int, greaterThan(0));

    // Another size on its own body.
    final shown = sizes.last as String;
    await tester.tap(find.text(grading['shown'] as String).last);
    await settle();
    await tester.tap(find.text(shown).last);
    await settle();
    expect((run({'type': 'grading.get'}) as Map)['shown'], shown);

    // The other sizes' outlines can be hidden on the board.
    expect(engine.showSizes(), isTrue);
    await tester.tap(find.byKey(const Key('sizes-on-board')));
    await settle();
    expect(engine.showSizes(), isFalse);

    // A grade typed on one corner of the front: click the corner on the
    // board (its view id is found by trying the few there are).
    final summary = run({'type': 'project.summary'}) as Map<String, dynamic>;
    final front = (summary['panels'] as List).firstWhere(
      (p) => p['name'] == 'Front',
    );
    final panel = run({'type': 'panel.get', 'panel': front['id']}) as Map;
    final corner = panel['edges'][0]['start'] as Map;
    final board = tester.getRect(
      find.byWidgetPredicate((w) => w is RustView && w.kind == ViewKind.board),
    );
    final ratio = tester.view.devicePixelRatio;
    for (var view = 0; view < 8; view++) {
      final px = engine.toView(
        view: view,
        xMm: (corner['pos'][0] as num).toDouble(),
        yMm: (corner['pos'][1] as num).toDouble(),
      );
      if (px == null) continue;
      final at = board.topLeft + Offset(px[0], px[1]) / ratio;
      if (!board.contains(at)) continue;
      await tester.tapAt(at);
      await settle();
      if (engine.selectedPoints().any((p) => p.id == corner['id'])) break;
    }
    expect(engine.selectedPoints().map((p) => p.id), contains(corner['id']));
    final x = find.byKey(const Key('grade-x'));
    await tester.scrollUntilVisible(
      x,
      100,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.enterText(
      find.descendant(of: x, matching: find.byType(TextField)),
      '-7',
    );
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await settle();
    final graded = run({'type': 'panel.get', 'panel': front['id']}) as Map;
    final grade = graded['edges'][0]['start']['grade'] as List;
    final base = sizes.indexOf(grading['base']);
    expect(grade[base + 1][0] - grade[base][0], closeTo(-7, 1e-9));

    // The pattern exports as its graded nest.
    engine.exportFile(
      format: ExportFormat.dxf,
      path: '$dir/nest.dxf',
      avatar: false,
    );
    final dxf = File('$dir/nest.dxf').readAsStringSync();
    for (final s in sizes) {
      expect(dxf, contains('Size: $s'));
    }
  });
}
