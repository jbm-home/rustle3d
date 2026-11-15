// The 3D view end to end: a click on the garment selects the piece under
// the cursor; a drag on it pulls the cloth (the simulation runs).

import 'dart:convert';
import 'dart:io';

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/main.dart';
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
    engine.runTemplate(name: 'a-line-skirt');
    expect(engine.resetSimulation().loaded, isTrue);

    await tester.pumpWidget(Rustle3dApp(engine: engine));
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
    expect(engine.simulationStatus().running, isFalse);

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
  });
}
