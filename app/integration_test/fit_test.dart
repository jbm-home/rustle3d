// Fit maps and the tape measure (spec 24): the gauge over the 3D view
// colours the cloth by strain and shows a legend; the Tape measure tool
// reads the circumferences and the ease where the garment is clicked, or
// a length between two points of it.

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

  testWidgets('a fit map with its legend, and the tape measure', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: await store.create(name: 'Tee', body: store.standardBody()),
    );
    engine.run(
      stepJson: jsonEncode({'type': 'template.apply', 'template': 't_shirt'}),
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

    // Strain: the cloth is coloured and the legend shows the scale.
    expect(engine.fitLegend(), isNull);
    await tester.tap(find.byKey(const ValueKey('fitButton')));
    await frames();
    await tester.tap(find.text('Strain').last);
    await frames();
    final legend = engine.fitLegend();
    expect(legend?.map, 'strain');
    expect(legend!.colors, hasLength(9));
    expect(legend.tight, 20);
    expect(find.byKey(const ValueKey('fitLegend')), findsOneWidget);
    expect(find.text('Strain: stretch beyond the pattern'), findsOneWidget);

    // Back to the fabric: no legend.
    await tester.tap(find.byKey(const ValueKey('fitButton')));
    await frames();
    await tester.tap(find.text('Fabric colours').last);
    await frames();
    expect(engine.fitLegend(), isNull);
    expect(find.byKey(const ValueKey('fitLegend')), findsNothing);

    // The tape measure: a click on the garment reads it.
    final tapeTool = find.byWidgetPredicate(
      (w) =>
          w is Tooltip &&
          (w.richMessage?.toPlainText().startsWith('Tape measure') ?? false),
    );
    await tester.ensureVisible(tapeTool);
    await frames(3);
    await tester.tap(tapeTool);
    await frames();
    expect(find.byKey(const ValueKey('tapeCard')), findsOneWidget);
    expect(find.text('Click on the draped garment.'), findsOneWidget);
    final view = find.byWidgetPredicate(
      (w) => w is RustView && w.kind == ViewKind.garment,
    );
    final rect = tester.getRect(view);
    // The shirt is around the middle of the body, a little above the
    // view's centre: try a few heights until the click lands on it.
    var onShirt = rect.center;
    for (final dy in [0.0, -0.05, -0.1, -0.15, 0.05]) {
      onShirt = rect.center + Offset(0, dy * rect.height);
      await tester.tapAt(onShirt);
      await frames();
      if (find.text('Ease').evaluate().isNotEmpty) break;
    }
    expect(find.text('Ease'), findsOneWidget);
    expect(find.text('Garment'), findsOneWidget);

    // A length: two points clicked on the shirt, a little apart.
    await tester.tap(find.text('Length'));
    await frames();
    expect(
      find.text('Click the first point on the draped garment.'),
      findsOneWidget,
    );
    await tester.tapAt(onShirt);
    await frames();
    expect(find.text('Click the second point.'), findsOneWidget);
    // The second point a little beside the first, still on the shirt.
    for (final dx in [0.03, -0.03, 0.015, -0.015]) {
      await tester.tapAt(onShirt + Offset(dx * rect.width, 0));
      await frames();
      if (find.text('Along the cloth').evaluate().isNotEmpty) break;
    }
    expect(find.text('Along the cloth'), findsOneWidget);
    expect(find.text('Straight'), findsOneWidget);

    // Another tool puts the tape away.
    await tester.tap(find.byTooltip('Put the tape away'));
    await frames();
    expect(find.byKey(const ValueKey('tapeCard')), findsNothing);
  });
}
