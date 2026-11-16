// Exports from the app: the pattern, and the garment as the 3D view shows
// it (the running drape).

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('the pattern and the draped garment export', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    final engine = Engine.open(
      path: store.create(
        name: 'Skirt',
        body: store.standardBody(),
        template: 'a_line_skirt',
      ),
    );
    engine.resetSimulation();
    engine.runSimulation(running: true);
    while (engine.simulationStatus().timeS < 0.2) {
      await tester.pump(const Duration(milliseconds: 50));
    }

    engine.exportFile(
      format: ExportFormat.glb,
      path: '$dir/a.glb',
      avatar: true,
    );
    final glb = File('$dir/a.glb').readAsBytesSync();
    expect(String.fromCharCodes(glb.sublist(0, 4)), 'glTF');
    // The avatar makes it much larger than the garment alone.
    engine.exportFile(
      format: ExportFormat.glb,
      path: '$dir/b.glb',
      avatar: false,
    );
    expect(glb.length, greaterThan(File('$dir/b.glb').lengthSync() * 2));

    engine.exportFile(
      format: ExportFormat.obj,
      path: '$dir/a.obj',
      avatar: false,
    );
    expect(File('$dir/a.obj').readAsStringSync(), contains('o Front'));
    engine.exportFile(
      format: ExportFormat.svg,
      path: '$dir/a.svg',
      avatar: false,
    );
    expect(File('$dir/a.svg').readAsStringSync(), contains('<svg'));

    expect(
      () => engine.exportFile(
        format: ExportFormat.svg,
        path: '$dir/missing/folder/a.svg',
        avatar: false,
      ),
      throwsA(isA<ApiError>().having((e) => e.code, 'code', 'write_failed')),
    );
    engine.runSimulation(running: false);
  });
}
