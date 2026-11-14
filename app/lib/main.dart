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
  runApp(Rustle3dApp(engine: engine));
}

class Rustle3dApp extends StatefulWidget {
  const Rustle3dApp({super.key, required this.engine});

  final Engine engine;

  @override
  State<Rustle3dApp> createState() => _Rustle3dAppState();
}

class _Rustle3dAppState extends State<Rustle3dApp> {
  bool _dark = false;

  void _toggleTheme() {
    setState(() => _dark = !_dark);
    widget.engine.setDark(dark: _dark);
  }

  @override
  Widget build(BuildContext context) {
    final palette = _dark ? Palette.dark : Palette.light;
    return Tokens(
      palette: palette,
      child: MaterialApp(
        title: 'Rustle3D',
        debugShowCheckedModeBanner: false,
        theme: buildTheme(palette, _dark ? Brightness.dark : Brightness.light),
        home: Shell(
          engine: widget.engine,
          dark: _dark,
          onToggleTheme: _toggleTheme,
        ),
      ),
    );
  }
}
