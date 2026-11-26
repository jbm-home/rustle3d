// The pattern library in the app (spec 32): a garment saved from its card
// is listed under "My patterns" and added again from the library, and a
// built-in pattern (jeans) is added to the open project.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/api/library.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('a garment is saved to the library and patterns are added', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final engine = Engine.open(path: '$dir/Skirt.r3d');
    engine.run(
      stepJson: jsonEncode({
        'type': 'avatar.set',
        'body': {
          'masculinity': 0,
          'height_cm': 168,
          'bust_cm': 90,
          'waist_cm': 73,
          'hips_cm': 98,
        },
      }),
    );
    await tester.runAsync(
      () => engine.runLong(
        stepJson: jsonEncode({
          'type': 'template.apply',
          'template': 'a_line_skirt',
        }),
      ),
    );
    List<dynamic> garments() => jsonDecode(
      engine.run(stepJson: '{"type":"project.summary"}'),
    )['garments'];
    expect(garments(), hasLength(1));

    File('$dir/tour-seen').createSync();
    await tester.pumpWidget(
      Rustle3dApp(
        store: ProjectStore(projectsDir: dir, libraryDir: dir),
        engine: engine,
      ),
    );
    Future<void> frames([int n = 12]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 50));
      }
    }

    await frames(20);
    for (
      var i = 0;
      i < 300 && find.byType(ModalBarrier).evaluate().isNotEmpty;
      i++
    ) {
      await tester.pump(const Duration(milliseconds: 100));
    }

    // Saved from the garment's card, under a name and a kind.
    final id = garments().first['id'] as String;
    final save = find.byKey(ValueKey('saveToLibrary:$id'));
    await tester.ensureVisible(save);
    await frames(2);
    await tester.tap(save);
    await frames();
    await tester.enterText(
      find.byKey(const ValueKey('librarySaveName')),
      'My skirt',
    );
    await tester.tap(find.byKey(const ValueKey('librarySaveDone')));
    await frames();
    final mine = jsonDecode(libraryPatterns(libraryDir: dir)) as List;
    expect(mine.map((m) => m['name']), ['My skirt']);
    expect(mine.first['kind'], 'tops');

    // Added back from the library: a second garment.
    final open = find.byKey(const ValueKey('openPatternLibrary'));
    await tester.ensureVisible(open);
    await frames(2);
    await tester.tap(open);
    await frames();
    expect(find.byKey(const ValueKey('libraryBuiltin:jeans')), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('libraryMine:My skirt')));
    for (var i = 0; i < 100 && garments().length < 2; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();
    }
    expect(garments(), hasLength(2));
    for (
      var i = 0;
      i < 300 && find.byType(ModalBarrier).evaluate().isNotEmpty;
      i++
    ) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();
    }

    // A built-in pattern: the jeans.
    await tester.ensureVisible(open);
    await frames(2);
    await tester.tap(open);
    await frames();
    final jeans = find.byKey(const ValueKey('libraryBuiltin:jeans'));
    await tester.ensureVisible(jeans);
    await frames(2);
    await tester.tap(jeans);
    for (var i = 0; i < 200 && garments().length < 3; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 100)),
      );
      await tester.pump();
    }
    expect(garments().map((g) => g['name']), contains('Jeans'));
  });

  testWidgets('the start screen opens the library to manage it', (
    tester,
  ) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    File('$dir/tour-seen').createSync();
    Directory('$dir/projects').createSync();
    await tester.pumpWidget(
      Rustle3dApp(
        store: ProjectStore(projectsDir: '$dir/projects', libraryDir: dir),
      ),
    );
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    await tester.tap(find.byKey(const ValueKey('libraryButton')));
    for (var i = 0; i < 12; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    // The user's patterns and Import; no built-in ones to add, with no
    // project open.
    expect(find.byKey(const ValueKey('libraryImport')), findsOneWidget);
    expect(find.byKey(const ValueKey('libraryBuiltin:jeans')), findsNothing);
    expect(find.textContaining('rename, export or delete'), findsOneWidget);
  });
}
