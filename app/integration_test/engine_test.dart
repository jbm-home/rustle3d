import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  test('the Rust engine runs steps and reports structured errors', () {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final engine = Engine.open(path: '$dir/P.r3d');
    final out = jsonDecode(engine.run(
      stepJson: jsonEncode({'type': 'panel.create_rect', 'width': 100, 'height': 200}),
    ));
    expect(out['created'][0], 'panel:2');
    expect(
      () => engine.run(stepJson: '{"type":"panel.explode"}'),
      throwsA(isA<ApiError>().having((e) => e.code, 'code', 'unknown_type')),
    );
  });
}
