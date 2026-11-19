// Imported bodies (spec 27): a body file imported into the project gets
// a Body menu with its check and arrangement points; the check window
// corrects how the file is read; motions are off; the built-in body comes
// back from the menu.

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

  testWidgets('import a body, check it, turn it, back to the built-in one', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final project = await store.create(
      name: 'Scan',
      body: store.standardBody(),
    );
    final engine = Engine.open(path: project);
    // A box 170 cm tall standing for a body, in centimetres.
    final obj = StringBuffer();
    for (var i = 0; i < 8; i++) {
      final x = i & 1 == 0 ? -20 : 20;
      final y = i & 2 == 0 ? 0 : 170;
      final z = i & 4 == 0 ? -12 : 13;
      obj.writeln('v $x $y $z');
    }
    obj.write(
      'f 1 3 4 2\nf 5 6 8 7\nf 1 2 6 5\nf 3 7 8 4\nf 1 5 7 3\nf 2 4 8 6\n',
    );
    final file = File('$dir/box.obj')..writeAsStringSync(obj.toString());
    final imported = jsonDecode(
      await engine.importBody(path: file.path),
    ) as Map<String, dynamic>;
    expect(imported['check']['scale'], 0.01);
    expect(File('$project/avatars/box.obj').existsSync(), isTrue);

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

    Map<String, dynamic> check() =>
        jsonDecode(engine.run(stepJson: '{"type": "avatar.check"}'))
            as Map<String, dynamic>;

    await idle();

    // Motions are off on an imported body.
    final motion = find.byKey(const ValueKey('motionButton'));
    expect(motion, findsOneWidget);
    expect(tester.widget<IconButton>(motion).onPressed, isNull);

    // The Body menu leads to the check.
    await tester.tap(find.byTooltip('Body'));
    await frames();
    expect(find.byKey(const Key('bodyMenuPoints')), findsOneWidget);
    await tester.tap(find.byKey(const Key('bodyMenuCheck')));
    await frames();
    expect(find.byKey(const Key('bodyCheckReady')), findsOneWidget);

    // A turn typed in the check window.
    await tester.enterText(find.byKey(const Key('bodyTurn')), '180');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await idle();
    expect(check()['turn_deg'], 180.0);
    await tester.tap(find.byIcon(Icons.close_rounded).last);
    await frames();

    // The points shown in 3D: some view draws them.
    await tester.tap(find.byTooltip('Body'));
    await frames();
    await tester.tap(find.byKey(const Key('bodyMenuPoints')));
    await frames();
    final drawn = [
      for (var view = 0; view < 8; view++) ...engine.bodyPoints(view: view),
    ];
    expect(drawn.length, 14);

    // Back to the built-in body.
    await tester.tap(find.byTooltip('Body'));
    await frames();
    await tester.tap(find.text('Use the built-in body'));
    await idle();
    expect(
      () => engine.run(stepJson: '{"type": "avatar.check"}'),
      throwsA(isA<ApiError>()),
    );
  });
}
