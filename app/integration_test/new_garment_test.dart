// The start screen's new-project window (docs/specs/36-objects.md,
// 37-body-at-creation.md): the body is chosen by who it dresses, a size
// and a silhouette, typed measurements say the size they make, the
// template's sizes follow the body without blocking the window; a blank
// project can be an object, which opens without a body. Sizes and lengths
// are written in the user's system and units (38-region.md).

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

  /// A fresh library in EU sizes and centimetres, whatever this
  /// machine's region.
  ProjectStore newStore() {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    File('$dir/tour-seen').createSync();
    File('$dir/region.json')
        .writeAsStringSync('{"sizes": "eu", "units": "metric"}');
    Directory('$dir/projects').createSync();
    return ProjectStore(projectsDir: '$dir/projects', libraryDir: dir);
  }

  /// Pump until [done], letting Rust's futures complete (they need real
  /// time, not the test's clock).
  Future<void> until(WidgetTester tester, bool Function() done) async {
    final end = DateTime.now().add(const Duration(seconds: 90));
    while (!done() && DateTime.now().isBefore(end)) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 50)),
      );
      await tester.pump(const Duration(milliseconds: 50));
    }
    expect(done(), isTrue);
  }

  final create = find.byKey(const Key('createProject'));
  bool ready(WidgetTester tester) =>
      create.evaluate().isNotEmpty &&
      tester.widget<ButtonStyleButton>(create).onPressed != null;

  testWidgets('the body is chosen by size and the sizes follow it', (
    tester,
  ) async {
    List<String> sizes() => [
      for (final f in tester.widgetList<TextField>(
        find.descendant(
          of: find.byKey(const Key('templateSizes')),
          matching: find.byType(TextField),
        ),
      ))
        f.controller!.text,
    ];
    String field(String key) => tester
        .widget<TextField>(find.byKey(Key('bodyField_$key')))
        .controller!
        .text;

    await tester.pumpWidget(Rustle3dApp(store: newStore()));
    await until(tester, () => find.text('T-shirt').evaluate().isNotEmpty);
    await tester.tap(find.text('T-shirt').first);
    await until(tester, () => ready(tester));
    // Women's 38 to start with, as the chart has it.
    expect(field('bust_cm'), '88');
    final first = sizes();
    expect(first, isNotEmpty);

    // Another size: its measurements, and the template's sizes for it.
    await tester.tap(find.byKey(const Key('bodySize')));
    // The menu says each size's letter beside it.
    final size44 = find.text('44   L', findRichText: true);
    await until(tester, () => size44.evaluate().isNotEmpty);
    await tester.tap(size44.last);
    await tester.pump();
    expect(field('bust_cm'), '100');
    await until(tester, () => ready(tester) && sizes().join() != first.join());

    // A pear shape keeps the bust and widens the hips.
    await tester.tap(find.byKey(const Key('silhouette_pear')));
    await tester.pump();
    expect((field('bust_cm'), field('hips_cm')), ('100', '114'));

    // Typed measurements say the size they make.
    await tester.enterText(find.byKey(const Key('bodyField_bust_cm')), '92');
    await tester.pump();
    expect(find.textContaining('make a size 40'), findsOneWidget);

    // A baby's body: the sizes follow, without a card over the window.
    await tester.tap(find.text('Babies'));
    await tester.pump();
    // No busy card over the window: the sizes come in the background.
    expect(find.text('Changing the body'), findsNothing);
    await until(tester, () => ready(tester));
    expect(sizes(), isNot(first));
  });

  testWidgets('sizes and lengths in US sizes and inches, remembered', (
    tester,
  ) async {
    Finder shownSize(String size) => find.descendant(
      of: find.byKey(const Key('bodySize')),
      matching: find.text(size),
    );
    String field(String key) => tester
        .widget<TextField>(find.byKey(Key('bodyField_$key')))
        .controller!
        .text;
    final store = newStore();
    await tester.pumpWidget(Rustle3dApp(store: store));
    await until(tester, () => find.text('T-shirt').evaluate().isNotEmpty);
    await tester.tap(find.text('T-shirt').first);
    await until(tester, () => ready(tester));
    expect(shownSize('38'), findsOneWidget);

    // US sizes: the women's 38 is a 6.
    await tester.tap(find.byKey(const Key('sizeSystem')));
    await tester.pump(const Duration(milliseconds: 400));
    await tester.tap(find.byKey(const Key('sizeSystem_us')));
    await tester.pump(const Duration(milliseconds: 400));
    expect(shownSize('6'), findsOneWidget);
    expect(find.text('US'), findsOneWidget);

    // Inches: the bust of 88 cm reads 34.6 in, and still makes the 38.
    await tester.tap(find.text('in'));
    await tester.pump();
    expect(field('bust_cm'), '34.6');
    await tester.enterText(find.byKey(const Key('bodyField_bust_cm')), '36.2');
    await tester.pump();
    // 92 cm: a 40, an 8 in US sizes.
    expect(find.textContaining('make a size 8'), findsOneWidget);

    // Remembered: the next window starts in US sizes and inches.
    final saved = File('${store.libraryDir}/region.json').readAsStringSync();
    expect(saved, contains('"us"'));
    expect(saved, contains('"imperial"'));
    await tester.pumpWidget(const SizedBox());
    await tester.pumpWidget(Rustle3dApp(store: store));
    await until(tester, () => find.text('T-shirt').evaluate().isNotEmpty);
    await tester.tap(find.text('T-shirt').first);
    await until(tester, () => ready(tester));
    expect(shownSize('6'), findsOneWidget);
    expect(field('height_cm'), '66.1');
  });

  testWidgets('a blank project made as an object has no body', (tester) async {
    await tester.pumpWidget(Rustle3dApp(store: newStore()));
    await until(tester, () => find.text('Blank project').evaluate().isNotEmpty);
    await tester.tap(find.text('Blank project'));
    await until(tester, () => ready(tester));
    await tester.tap(find.byKey(const Key('kindObject')));
    await tester.pump();
    expect(find.byKey(const Key('stuffing')), findsOneWidget);
    expect(find.byKey(const Key('bodyChart')), findsNothing);
    await tester.tap(create);
    // The project opens with an Object button where the Body menu was.
    await until(
      tester,
      () => find.widgetWithText(TextButton, 'Object').evaluate().isNotEmpty,
    );
    expect(find.byKey(const Key('bodyMenuImport')), findsNothing);
  });
}
