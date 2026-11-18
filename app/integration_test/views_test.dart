// Maximized views: the board or the 3D view takes the space of both; the
// drape pauses while the 3D view is hidden and goes on when it is back.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart' show ValueKey;
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

  testWidgets('maximize a view, the drape pauses and goes on', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final engine = Engine.open(path: '$dir/Skirt.r3d');
    engine.run(
      stepJson: jsonEncode({
        'type': 'template.apply',
        'template': 'a_line_skirt',
      }),
    );
    File('$dir/tour-seen').createSync();
    await tester.pumpWidget(
      Rustle3dApp(
        store: ProjectStore(projectsDir: dir, libraryDir: dir),
        engine: engine,
      ),
    );
    Future<void> settle() async {
      for (var i = 0; i < 10; i++) {
        await tester.pump(const Duration(milliseconds: 50));
      }
    }

    await settle();
    Finder view(ViewKind kind) =>
        find.byWidgetPredicate((w) => w is RustView && w.kind == kind);
    final board = view(ViewKind.board);
    final garment = view(ViewKind.garment);
    final boardWidth = tester.getSize(board).width;

    await engine.runSimulation(running: true);
    await settle();

    // The board takes both views' space; the hidden drape pauses.
    await tester.tap(find.byKey(const ValueKey('maximizeView')).first);
    await settle();
    expect(garment, findsNothing);
    expect(tester.getSize(board).width, greaterThan(boardWidth * 1.8));
    expect(engine.simulationStatus().running, isFalse);

    // Back to both views: the drape goes on.
    await tester.tap(find.byKey(const ValueKey('restoreViews')));
    await settle();
    expect(garment, findsOneWidget);
    expect(engine.simulationStatus().running, isTrue);

    // The 3D view maximized keeps draping; M brings both views back.
    await tester.tap(find.byKey(const ValueKey('maximizeView')).last);
    await settle();
    expect(board, findsNothing);
    expect(engine.simulationStatus().running, isTrue);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyM);
    await settle();
    expect(board, findsOneWidget);
    expect(garment, findsOneWidget);

    // A drape paused by the user stays paused when the views come back.
    await engine.runSimulation(running: false);
    await tester.tap(find.byKey(const ValueKey('maximizeView')).first);
    await settle();
    await tester.tap(find.byKey(const ValueKey('restoreViews')));
    await settle();
    expect(engine.simulationStatus().running, isFalse);
  });
}
