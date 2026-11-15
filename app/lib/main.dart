// Rustle3D desktop app: a Flutter UI over the Rust core (crates/r3d-ffi).

import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';
import 'package:path_provider/path_provider.dart';

import 'src/design/tokens.dart';
import 'src/rust/api/engine.dart';
import 'src/rust/frb_generated.dart';
import 'src/shell.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final engine = Engine.open(
    path: '${Directory.systemTemp.path}/rustle3d-spike.r3d',
  );
  final summary = jsonDecode(
    engine.run(stepJson: '{"type":"project.summary"}'),
  );
  if ((summary['counts']['panels'] as int) == 0) {
    engine.runScript(
      scriptJson: await rootBundle.loadString('assets/demo_skirt.json'),
    );
  }
  engine.setLibraryDir(path: (await getApplicationSupportDirectory()).path);
  // Every project starts with a body to dress: a standard size.
  if (jsonDecode(engine.run(stepJson: '{"type":"avatar.get"}')) == null) {
    final presets = jsonDecode(engine.bodyPresets()) as List;
    final standard = presets.firstWhere((p) => p['name'] == 'Women M');
    engine.run(
      stepJson: jsonEncode({
        'type': 'avatar.set',
        'body': standard['body'],
        'preset': standard['name'],
      }),
    );
  }
  // Review aids for screenshots:
  // --dart-define=R3D_DARK=true, R3D_SELECT=panel:2 (scripts/app-screenshot.sh).
  const select = String.fromEnvironment('R3D_SELECT');
  if (select.isNotEmpty) engine.select(panels: [select]);
  runApp(Rustle3dApp(engine: engine));
}

class Rustle3dApp extends StatefulWidget {
  const Rustle3dApp({super.key, required this.engine});

  final Engine engine;

  @override
  State<Rustle3dApp> createState() => _Rustle3dAppState();
}

class _Rustle3dAppState extends State<Rustle3dApp> {
  bool _dark = const bool.fromEnvironment('R3D_DARK');

  @override
  void initState() {
    super.initState();
    _applyCanvas();
  }

  void _applyCanvas() => widget.engine.setCanvas(
    colors: (_dark ? Palette.dark : Palette.light).toCanvas(),
  );

  void _toggleTheme() {
    setState(() => _dark = !_dark);
    _applyCanvas();
  }

  @override
  Widget build(BuildContext context) {
    return Tokens(
      dark: _dark,
      child: MaterialApp(
        title: 'Rustle3D',
        debugShowCheckedModeBanner: false,
        theme: buildTheme(_dark),
        home: Shell(
          engine: widget.engine,
          dark: _dark,
          onToggleTheme: _toggleTheme,
        ),
      ),
    );
  }
}
