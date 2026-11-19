// Renaming a piece from its inspector: the pencil turns the name into a
// field; Enter, the tick or a click elsewhere renames (one undoable step),
// Esc keeps the name.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
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

  testWidgets('rename a piece from its inspector', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final engine = Engine.open(path: '$dir/Rename.r3d');
    final created = jsonDecode(
      engine.run(
        stepJson: jsonEncode({
          'type': 'panel.create_rect',
          'name': 'Front',
          'width': 400,
          'height': 600,
        }),
      ),
    );
    final panel = (created['created'] as List).first as String;
    String name() {
      final summary = jsonDecode(
        engine.run(stepJson: '{"type":"project.summary"}'),
      );
      return (summary['panels'] as List).first['name'] as String;
    }

    // Selected before the app opens: its inspector shows from the start.
    engine.select(panels: [panel]);
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
    final pencil = find.byKey(const Key('piece-rename'));
    expect(pencil, findsOneWidget);

    // Esc keeps the name.
    await tester.tap(pencil);
    await frames(4);
    await tester.enterText(find.byKey(const Key('piece-name-field')), 'Nope');
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await frames(4);
    expect(name(), 'Front');
    expect(find.byKey(const Key('piece-name-field')), findsNothing);

    // Enter renames; undo brings the old name back.
    await tester.tap(pencil);
    await frames(4);
    await tester.enterText(
      find.byKey(const Key('piece-name-field')),
      '  Left front ',
    );
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await frames();
    expect(name(), 'Left front');
    expect(find.text('Left front'), findsWidgets);
    engine.run(stepJson: '{"type":"history.undo"}');
    expect(name(), 'Front');
    await frames();

    // Enter is the field's, not the window's (which finishes a drawing):
    // the key is left to the text input, which submits.
    await tester.tap(pencil);
    await frames(4);
    final field = find.byKey(const Key('piece-name-field'));
    await tester.enterText(field, 'Back');
    expect(
      await tester.sendKeyEvent(LogicalKeyboardKey.enter),
      isFalse,
      reason: 'a window shortcut took Enter',
    );
    await frames(4);

    // The tick renames.
    await tester.enterText(field, 'Back');
    await tester.tap(find.byKey(const Key('piece-name-ok')));
    await frames();
    expect(name(), 'Back');

    // So does a click elsewhere in the inspector.
    await tester.tap(pencil);
    await frames(4);
    await tester.enterText(field, 'Yoke');
    await tester.tapAt(tester.getBottomLeft(field) + const Offset(4, 80));
    await frames();
    expect(name(), 'Yoke');
    expect(find.byKey(const Key('piece-name-field')), findsNothing);
  });
}
