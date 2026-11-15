// Rustle3D desktop app: a Flutter UI over the Rust core (crates/r3d-ffi).

import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

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
  // Review aids while choosing a visual direction (removed afterwards):
  // --dart-define=R3D_STYLE=Studio|Soft, R3D_DARK=true, R3D_SELECT=panel:2.
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
  AppStyle _style = AppStyle.all.firstWhere(
    (s) => s.name == const String.fromEnvironment('R3D_STYLE'),
    orElse: () => AppStyle.atelier,
  );
  bool _dark = const bool.fromEnvironment('R3D_DARK');

  @override
  void initState() {
    super.initState();
    _applyCanvas();
  }

  void _applyCanvas() =>
      widget.engine.setCanvas(colors: _style.palette(_dark).toCanvas());

  void _toggleTheme() {
    setState(() => _dark = !_dark);
    _applyCanvas();
  }

  // Temporary while choosing a visual direction.
  void _cycleStyle() {
    final all = AppStyle.all;
    setState(() => _style = all[(all.indexOf(_style) + 1) % all.length]);
    _applyCanvas();
  }

  @override
  Widget build(BuildContext context) {
    return Tokens(
      style: _style,
      dark: _dark,
      child: MaterialApp(
        title: 'Rustle3D',
        debugShowCheckedModeBanner: false,
        theme: buildTheme(_style, _dark),
        home: Shell(
          engine: widget.engine,
          dark: _dark,
          onToggleTheme: _toggleTheme,
          styleName: _style.name,
          onCycleStyle: _cycleStyle,
        ),
      ),
    );
  }
}
