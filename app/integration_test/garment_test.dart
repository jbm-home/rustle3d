// The 3D view end to end: a click on the garment selects the piece under
// the cursor; a drag on it pulls the cloth (the simulation runs).

import 'dart:convert';
import 'dart:io';

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
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

  testWidgets('click selects a piece, drag pulls the cloth', (tester) async {
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
    engine.run(
      stepJson: jsonEncode({
        'type': 'template.apply',
        'template': 'a_line_skirt',
      }),
    );
    expect(engine.resetSimulation().loaded, isTrue);

    await tester.pumpWidget(
      Rustle3dApp(
        store: ProjectStore(projectsDir: dir, libraryDir: dir),
        engine: engine,
      ),
    );
    // Let the views attach their textures and draw.
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    final garment = find.byWidgetPredicate(
      (w) => w is RustView && w.kind == ViewKind.garment,
    );
    expect(garment, findsOneWidget);
    // The default camera looks at the hips: the skirt is at the centre.
    final centre = tester.getCenter(garment);

    await tester.tapAt(centre);
    await tester.pump();
    expect(engine.selection(), hasLength(1));
    expect(engine.selectedEdge(), isNull);
    expect(engine.simulationStatus().running, isFalse);

    // Clicking near the hem picks the edge as well (shown in both views).
    for (var dy = 60.0; dy < 300 && engine.selectedEdge() == null; dy += 2) {
      await tester.tapAt(centre + Offset(0, dy));
      await tester.pump();
    }
    expect(engine.selectedEdge(), isNotNull);
    expect(engine.selection(), hasLength(1));

    final gesture = await tester.startGesture(
      centre,
      kind: PointerDeviceKind.mouse,
    );
    for (var i = 0; i < 10; i++) {
      await gesture.moveBy(const Offset(12, -6));
      await tester.pump(const Duration(milliseconds: 30));
    }
    expect(engine.simulationStatus().running, isTrue);
    await gesture.up();
    await tester.pump();

    // ⌥-click pins the cloth (a pin.add, listed) instead of selecting.
    engine.select(panels: []);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.altLeft);
    await tester.tapAt(centre);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.altLeft);
    await tester.pump();
    final pins =
        jsonDecode(engine.run(stepJson: '{"type":"pin.list"}')) as List;
    expect(pins, hasLength(1));
    expect(engine.selection(), isEmpty);
    expect(find.text('Pins'), findsOneWidget);

    engine.unpin(pin: pins.first['id'] as String);
    expect(jsonDecode(engine.run(stepJson: '{"type":"pin.list"}')), isEmpty);

    // ⇧-drag down on the hem lengthens the skirt in the pattern, as one
    // undoable step. The hem is somewhere below the centre of the view.
    double frontBottom() {
      final summary = jsonDecode(
        engine.run(stepJson: '{"type":"project.summary"}'),
      );
      final front = (summary['panels'] as List).firstWhere(
        (p) => p['name'] == 'Front',
      );
      return (front['bbox']['min'][1] as num).toDouble();
    }

    final before = frontBottom();
    for (var dy = 60.0; dy < 260 && frontBottom() == before; dy += 8) {
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      final drag = await tester.startGesture(
        centre + Offset(0, dy),
        kind: PointerDeviceKind.mouse,
      );
      for (var i = 0; i < 8; i++) {
        await drag.moveBy(const Offset(0, 5));
        await tester.pump(const Duration(milliseconds: 16));
      }
      await drag.up();
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.pump();
    }
    expect(frontBottom(), lessThan(before - 20));
    engine.run(stepJson: '{"type":"history.undo"}');
    expect(frontBottom(), before);

    // ⇧-drag on a seam takes it in or lets it out (towards or away from
    // the body; the direction itself is tested on the command): the
    // pattern's area changes, the seams stay matched, one undo restores.
    double width() {
      final summary = jsonDecode(
        engine.run(stepJson: '{"type":"project.summary"}'),
      );
      return (summary['panels'] as List).fold<double>(
        0,
        (sum, p) => sum + (p['area_mm2'] as num),
      );
    }

    final wide = width();
    for (var dx = -160.0; dx <= 160 && width() == wide; dx += 6) {
      await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
      final drag = await tester.startGesture(
        centre + Offset(dx, -20),
        kind: PointerDeviceKind.mouse,
      );
      for (var i = 0; i < 6; i++) {
        await drag.moveBy(const Offset(4, 0));
        await tester.pump(const Duration(milliseconds: 16));
      }
      await drag.up();
      await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
      await tester.pump();
    }
    expect((width() - wide).abs(), greaterThan(100));
    final seams =
        jsonDecode(engine.run(stepJson: '{"type":"seam.list"}')) as List;
    for (final seam in seams) {
      expect(seam['mismatch_mm'] as num, lessThan(0.5));
    }
    engine.run(stepJson: '{"type":"history.undo"}');
    expect(width(), closeTo(wide, 1e-6));

    // Draw on garment (D): a stroke across the skirt becomes an internal
    // line on the pattern.
    int lines() {
      final summary = jsonDecode(
        engine.run(stepJson: '{"type":"project.summary"}'),
      );
      return (summary['panels'] as List).fold<int>(0, (n, p) {
        final panel = jsonDecode(
          engine.run(
            stepJson: jsonEncode({'type': 'panel.get', 'panel': p['id']}),
          ),
        );
        return n + (panel['internal_lines'] as List).length;
      });
    }

    final linesBefore = lines();
    await tester.sendKeyEvent(LogicalKeyboardKey.keyD);
    await tester.pump();
    final stroke = await tester.startGesture(
      centre + const Offset(-20, 0),
      kind: PointerDeviceKind.mouse,
    );
    for (var i = 0; i < 10; i++) {
      await stroke.moveBy(const Offset(0, 8));
      await tester.pump(const Duration(milliseconds: 16));
    }
    await stroke.up();
    await tester.pump();
    // One line, or two when drawn on a piece with a linked mirror copy.
    expect(lines(), greaterThan(linesBefore));
    engine.run(stepJson: '{"type":"history.undo"}');
    expect(lines(), linesBefore);
  });
}
