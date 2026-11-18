// Whole projects from the start screen: rename one, delete one (only once
// the box is ticked), and a project packed into a .zip comes back from it.

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

  ProjectStore newStore() {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    File('$dir/tour-seen').createSync();
    Directory('$dir/projects').createSync();
    return ProjectStore(projectsDir: '$dir/projects', libraryDir: dir);
  }

  Future<void> openMenu(WidgetTester tester, String project) async {
    final card = find.ancestor(
      of: find.text(project),
      matching: find.byType(Column),
    );
    await tester.tap(
      find.descendant(
        of: card.first,
        matching: find.byTooltip('Project options'),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('rename and delete a project from its card', (tester) async {
    final store = newStore();
    final skirt = await store.create(name: 'Skirt', body: store.standardBody());
    await store.create(name: 'Old draft', body: store.standardBody());

    await tester.pumpWidget(Rustle3dApp(store: store));
    await tester.pumpAndSettle();

    await openMenu(tester, 'Skirt');
    await tester.tap(find.text('Rename…'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Summer skirt');
    await tester.tap(find.widgetWithText(FilledButton, 'Rename'));
    await tester.pumpAndSettle();
    expect(find.text('Summer skirt'), findsOneWidget);
    expect(Directory(skirt).existsSync(), isFalse);
    final renamed = store.all().firstWhere((p) => p.name == 'Summer skirt');
    expect(renamed.path, endsWith('Summer skirt.r3d'));

    await openMenu(tester, 'Old draft');
    await tester.tap(find.text('Delete…'));
    await tester.pumpAndSettle();
    final delete = find.widgetWithText(FilledButton, 'Delete');
    expect(tester.widget<FilledButton>(delete).onPressed, isNull);
    await tester.tap(find.byType(Checkbox));
    await tester.pump();
    expect(tester.widget<FilledButton>(delete).onPressed, isNotNull);
    await tester.tap(delete);
    await tester.pumpAndSettle();
    expect(find.text('Old draft'), findsNothing);
    expect(store.all().map((p) => p.name), ['Summer skirt']);
  });

  testWidgets('a project exported as .zip imports back', (tester) async {
    final store = newStore();
    final path = await store.create(
      name: 'Tote',
      body: store.standardBody(),
      template: 'tote_bag',
    );
    final zip = '${store.libraryDir}/Tote.zip';
    await store.exportZip(path, zip);

    final copy = await store.importZip(zip);
    expect(copy, endsWith('Tote 2.r3d'));
    final engine = Engine.open(path: copy);
    expect(
      engine.run(stepJson: '{"type":"project.summary"}'),
      contains('"name":"Tote"'),
    );
    engine.dispose();

    await expectLater(
      store.importZip('${store.libraryDir}/tour-seen'),
      throwsA(isA<ApiError>().having((e) => e.code, 'code', 'archive.invalid')),
    );
  });
}
