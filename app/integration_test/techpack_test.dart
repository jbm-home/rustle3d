// Production, second half: the garment's notes typed under it, the tech
// pack written from the app, and the pattern projected at 1:1 (mirror,
// pan, calibration remembered).

import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projector.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('notes, the tech pack and the projector', (tester) async {
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

    // Nothing selected: the notes typed under the garment, saved when the
    // field is left.
    final notes = find.byKey(const Key('garment-notes'));
    await tester.scrollUntilVisible(
      notes,
      200,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.enterText(notes, 'Press the darts towards the middle.');
    await settle(2);
    FocusManager.instance.primaryFocus?.unfocus();
    await settle();
    final summary = run({'type': 'project.summary'}) as Map;
    expect(
      summary['garments'][0]['notes'],
      'Press the darts towards the middle.',
    );

    // The tech pack, its views rendered from the 3D view (the save
    // dialog is the system's: write directly).
    await engine.exportTechpack(
      path: '$dir/techpack.pdf',
      labelsJson: jsonEncode({'notes': 'Notes de montage'}),
      letter: false,
    );
    final pdf = File('$dir/techpack.pdf').readAsBytesSync();
    expect(String.fromCharCodes(pdf.take(4)), '%PDF');
    expect(latin1.decode(pdf).contains('Notes de montage'), isTrue);

    // Export → Projector, with the first piece selected.
    final panels = (summary['panels'] as List)
        .map((p) => p['id'] as String)
        .toList();
    engine.select(panels: [panels.first]);
    final drawing =
        jsonDecode(engine.projectorDrawing(panels: [panels.first])) as Map;
    expect((drawing['lines'] as List), isNotEmpty);
    Future<ProjectorPageState> openProjector() async {
      await tester.tap(find.byIcon(Icons.ios_share_rounded));
      await settle();
      await tester.tap(find.byKey(const Key('export-projector')));
      await settle();
      expect(find.byType(ProjectorPage), findsOneWidget);
      return tester.state<ProjectorPageState>(find.byType(ProjectorPage));
    }

    var projector = await openProjector();
    expect(projector.settings.mirror, isFalse);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyM);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await settle(2);
    expect(projector.settings.mirror, isTrue);
    expect(projector.pan.dx, 10);

    // Calibrate: the 20 cm square measured 22 cm a side.
    final before = projector.settings.pxPerMmX;
    await tester.tap(find.byKey(const Key('projector-calibrate')));
    await settle(2);
    await tester.enterText(
      find.byKey(const Key('projector-measured-width')),
      '22',
    );
    await tester.enterText(
      find.byKey(const Key('projector-measured-height')),
      '22',
    );
    await tester.tap(find.byKey(const Key('projector-apply')));
    await settle(2);
    expect(projector.settings.pxPerMmX, closeTo(before * 20 / 22, 1e-9));

    // Escape closes it; the next one remembers the room's settings.
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await settle();
    expect(find.byType(ProjectorPage), findsNothing);
    projector = await openProjector();
    expect(projector.settings.mirror, isTrue);
    expect(projector.settings.pxPerMmX, closeTo(before * 20 / 22, 1e-9));
    await tester.tap(find.byKey(const Key('projector-close')));
    await settle();
    expect(find.byType(ProjectorPage), findsNothing);
  });
}
