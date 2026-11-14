// Plumbing check for the M3 spike: Dart -> Rust engine -> project on disk.
// Replaced by the real shell once the texture bridge is in place.

import 'dart:convert';
import 'dart:io';

import 'package:material_ui/material_ui.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
  final dir = '${Directory.systemTemp.path}/rustle3d-spike.r3d';
  final engine = Engine.open(path: dir);
  engine.run(stepJson: jsonEncode({'type': 'panel.create_rect', 'width': 400, 'height': 600}));
  runApp(SpikeApp(engine: engine));
}

class SpikeApp extends StatelessWidget {
  const SpikeApp({super.key, required this.engine});

  final Engine engine;

  @override
  Widget build(BuildContext context) {
    final panels = jsonDecode(engine.run(stepJson: '{"type":"panel.list"}')) as List<dynamic>;
    return MaterialApp(
      home: Scaffold(
        body: Center(child: Text('Rustle3D engine: ${panels.length} panel(s)')),
      ),
    );
  }
}
