// The game-engine export from the app (spec 15): the running drape with
// levels of detail, a normal map and the skin, written where the user
// chose.

import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('the running drape exports for a game engine', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    final engine = Engine.open(
      path: await store.create(
        name: 'Tee',
        body: store.standardBody(),
        template: 't_shirt',
      ),
    );
    await engine.resetSimulation();
    await engine.runSimulation(running: true);
    while (engine.simulationStatus().timeS < 0.2) {
      await tester.pump(const Duration(milliseconds: 50));
    }

    final out = jsonDecode(
      await engine.exportGame(
        path: '$dir/tee.glb',
        optionsJson: jsonEncode({
          'preset': 'godot',
          'lods': [1, 0.5],
          'normal_map': 256,
          'avatar': true,
        }),
      ),
    ) as Map<String, dynamic>;
    expect((out['lods'] as List).length, 2);
    expect(out['skinned'], true);
    expect(out['animations'], ['walk', 'turn', 'raise_arms', 'idle']);
    final glb = File('$dir/tee.glb').readAsBytesSync();
    expect(String.fromCharCodes(glb.sublist(0, 4)), 'glTF');

    await expectLater(
      () => engine.exportGame(
        path: '$dir/bad.glb',
        optionsJson: jsonEncode({'lods': [0.5]}),
      ),
      throwsA(isA<ApiError>().having((e) => e.code, 'code', 'invalid.lods')),
    );
    await engine.runSimulation(running: false);
  });
}
