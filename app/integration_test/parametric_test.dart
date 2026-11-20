// A Seamly2D/Valentina pattern in the app (spec 28): imported with its
// measurement file, its garment says where it was drafted from, and
// "Draft again for the avatar" drafts it for a new body.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('an imported pattern is drafted again for the avatar', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final engine = Engine.open(path: '$dir/Skirt.r3d');
    final skirt = File('../crates/r3d-commands/tests/data/parametric/skirt.val')
        .absolute
        .path;
    // A real asynchronous call: outside the test's fake clock.
    final imported = await tester.runAsync(
      () => engine.importPattern(path: skirt),
    );
    final out = jsonDecode(imported!) as Map<String, dynamic>;
    expect(out['panels'], hasLength(1));

    double width() {
      final summary = jsonDecode(
        engine.run(stepJson: '{"type":"project.summary"}'),
      );
      final front = (summary['panels'] as List).firstWhere(
        (p) => p['name'] == 'Front',
      );
      final bbox = front['bbox'];
      return (bbox['max'][0] as num).toDouble() -
          (bbox['min'][0] as num).toDouble();
    }

    // Drafted for the measurement file: a quarter of hips 96 cm + 1 cm.
    expect(width(), closeTo(250, 1e-6));

    File('$dir/tour-seen').createSync();
    await tester.pumpWidget(
      Rustle3dApp(
        store: ProjectStore(projectsDir: dir, libraryDir: dir),
        engine: engine,
      ),
    );
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    for (
      var i = 0;
      i < 300 && find.byType(ModalBarrier).evaluate().isNotEmpty;
      i++
    ) {
      await tester.pump(const Duration(milliseconds: 100));
    }

    expect(find.text('Drafted from skirt.val'), findsOneWidget);
    final redraft = find.text('Draft again for the avatar');
    expect(redraft, findsOneWidget);

    // A new avatar, then the garment drafted again for it.
    engine.run(
      stepJson: jsonEncode({
        'type': 'avatar.set',
        'body': {
          'masculinity': 0,
          'height_cm': 168,
          'bust_cm': 92,
          'waist_cm': 80,
          'hips_cm': 104,
        },
      }),
    );
    await tester.pump();
    await tester.ensureVisible(redraft);
    await tester.pump();
    await tester.tap(redraft);
    // The step runs off the UI thread: wait in real time.
    for (var i = 0; i < 100 && width() < 260; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();
    }
    expect(width(), closeTo(270, 1e-6));
  });
}
