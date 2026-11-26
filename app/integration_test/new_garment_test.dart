// The start screen's new-garment window: choosing another body fits it
// (a second or more) off the UI thread, behind a card, and then shows the
// template's sizes for that body.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('another body is fitted behind a card', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    File('$dir/tour-seen').createSync();
    Directory('$dir/projects').createSync();
    final store = ProjectStore(projectsDir: '$dir/projects', libraryDir: dir);

    // The bar animates: pumpAndSettle would never return.
    Future<void> until(bool Function() done) async {
      final end = DateTime.now().add(const Duration(seconds: 60));
      while (!done() && DateTime.now().isBefore(end)) {
        await tester.pump(const Duration(milliseconds: 50));
      }
      expect(done(), isTrue);
    }

    final create = find.ancestor(
      of: find.text('Create'),
      matching: find.byWidgetPredicate((w) => w is ButtonStyleButton),
    );
    bool ready() =>
        create.evaluate().isNotEmpty &&
        tester.widget<ButtonStyleButton>(create).onPressed != null;
    List<String> sizes() => [
      for (final f in tester.widgetList<TextField>(
        find.descendant(
          of: find.byType(Wrap),
          matching: find.byType(TextField),
        ),
      ))
        f.controller!.text,
    ];
    final card = find.text('Changing the body');

    await tester.pumpWidget(Rustle3dApp(store: store));
    await until(() => find.text('T-shirt').evaluate().isNotEmpty);
    await tester.tap(find.text('T-shirt').first);
    await until(ready);
    final adult = sizes();
    expect(adult, isNotEmpty);

    await tester.tap(find.byType(DropdownButtonFormField<String>));
    await until(() => find.text('Baby 12 months').evaluate().isNotEmpty);
    await tester.tap(find.text('Baby 12 months').last);
    await until(() => card.evaluate().isNotEmpty);
    expect(find.text('Fitting the body to the measurements…'), findsOneWidget);
    await until(() => card.evaluate().isEmpty);
    expect(ready(), isTrue);
    expect(sizes(), isNot(adult));
  });
}
