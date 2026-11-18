// Production from the app: the fabric's price typed under the garment, a
// piece's cut area and mass, the marker dialog laying two sizes out with
// their cost, and the marker written as PDF.

import 'dart:convert';
import 'dart:io';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:intl/intl.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('fabric price, a marker of two sizes, its cost', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    final engine = Engine.open(
      path: await store.create(
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
    run({
      'type': 'grading.use_chart',
      'chart': 'women',
      'from': '36',
      'to': '44',
      'base': '40',
    });
    // The views draw every frame: pump a while instead of settling.
    Future<void> settle([int frames = 12]) async {
      for (var i = 0; i < frames; i++) {
        await tester.pump(const Duration(milliseconds: 50));
      }
    }

    File('$dir/tour-seen').createSync();
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    for (var i = 0; i < 400; i++) {
      await settle(1);
      if (find.byType(ModalBarrier).evaluate().isEmpty) break;
    }
    await settle();

    // Nothing selected: the price typed under the garment.
    final price = find.byKey(const Key('fabric-price'));
    await tester.scrollUntilVisible(
      price,
      200,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.enterText(
      find.descendant(of: price, matching: find.byType(TextField)),
      '12.5',
    );
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await settle();
    final figures = run({'type': 'production.indicators'}) as Map;
    final garment = figures['garments'][0] as Map;
    expect(garment['fabric_details']['price_per_m'], 12.5);
    expect(garment['totals']['mass_g'] as num, greaterThan(0));
    expect(find.textContaining('of fabric cut'), findsOneWidget);

    // Export → Marker: size 40 (shown in 3D) laid out, then 44 added.
    await tester.tap(find.byIcon(Icons.ios_share_rounded));
    await settle();
    await tester.tap(find.byKey(const Key('export-marker')));
    Future<void> laidOut() async {
      await settle(4);
      for (var i = 0; i < 200; i++) {
        await settle(1);
        if (find.byType(CircularProgressIndicator).evaluate().isEmpty) break;
      }
      await settle(4);
    }

    await laidOut();
    expect(find.text('Fabric used'), findsOneWidget);
    await tester.tap(find.byKey(const Key('marker-size-44')));
    await laidOut();
    final marker =
        jsonDecode(
              await engine.marker(
                specJson: jsonEncode({
                  'sizes': ['40', '44'],
                }),
              ),
            )
            as Map;
    expect((marker['pieces'] as List).length, greaterThan(2));
    final cost = NumberFormat.decimalPatternDigits(
      locale: 'en',
      decimalDigits: 2,
    ).format(marker['cost'] as num);
    expect(find.text(cost), findsOneWidget);

    // Saved as PDF (the save dialog is the system's: write directly).
    await engine.exportMarker(
      specJson: jsonEncode({
        'sizes': ['40', '44'],
      }),
      path: '$dir/marker.pdf',
      pdf: true,
    );
    expect(
      String.fromCharCodes(File('$dir/marker.pdf').readAsBytesSync().take(4)),
      '%PDF',
    );
    await tester.tap(find.text('Close'));
    await settle();
  });
}
