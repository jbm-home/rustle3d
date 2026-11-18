// Rustle3D desktop app: a Flutter UI over the Rust core (crates/r3d-ffi).

import 'dart:io';

import 'package:material_ui/material_ui.dart';

import 'src/design/tokens.dart';
import 'src/i18n.dart';
import 'src/projects.dart';
import 'src/rust/api/engine.dart';
import 'src/rust/frb_generated.dart';
import 'src/shell.dart';
import 'src/start_screen.dart';
import 'src/views/rust_view.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  await prepareSharedTextures();
  final store = await ProjectStore.open();
  // Review aids for screenshots (scripts/app-screenshot.sh):
  // --dart-define=R3D_DEMO=t_shirt opens a fresh project from that template,
  // R3D_OPEN=<folder> an existing project (made with the CLI);
  // R3D_DARK=true, R3D_SELECT=panel:2, R3D_SIMULATE=true.
  const demo = String.fromEnvironment('R3D_DEMO');
  const open = String.fromEnvironment('R3D_OPEN');
  Engine? engine;
  if (open.isNotEmpty) engine = Engine.open(path: open);
  if (demo.isNotEmpty) {
    final scratch = ProjectStore(
      projectsDir: '${Directory.systemTemp.path}/rustle3d-demo',
      libraryDir: store.libraryDir,
    );
    Directory(scratch.projectsDir).createSync(recursive: true);
    engine = Engine.open(
      path: await scratch.create(
        name: 'Demo',
        body: scratch.standardBody(),
        template: demo,
      ),
    );
  }
  if (engine != null) {
    const select = String.fromEnvironment('R3D_SELECT');
    if (select.isNotEmpty) engine.select(panels: [select]);
    if (const bool.fromEnvironment('R3D_SIMULATE')) {
      await engine.runSimulation(running: true);
    }
  }
  runApp(Rustle3dApp(store: store, engine: engine));
}

class Rustle3dApp extends StatefulWidget {
  const Rustle3dApp({super.key, required this.store, this.engine});

  final ProjectStore store;

  /// A project to open at once instead of the start screen.
  final Engine? engine;

  @override
  State<Rustle3dApp> createState() => _Rustle3dAppState();
}

class _Rustle3dAppState extends State<Rustle3dApp> {
  bool _dark = const bool.fromEnvironment('R3D_DARK');
  late Engine? _engine = widget.engine;
  late final _languageMemory = LanguageMemory(widget.store.libraryDir);
  late String _language = _languageMemory.code;

  void _setLanguage(String code) {
    _languageMemory.code = code;
    setState(() => _language = code);
  }

  @override
  void initState() {
    super.initState();
    _setCanvas(_engine);
  }

  /// Draw a project's views in the current theme. (The shell then places
  /// the garment around the body, ready to simulate.)
  void _setCanvas(Engine? engine) {
    engine?.setCanvas(
      colors: (_dark ? Palette.dark : Palette.light).toCanvas(),
    );
  }

  void _toggleTheme() {
    setState(() => _dark = !_dark);
    _setCanvas(_engine);
  }

  void _open(String path) {
    try {
      final engine = Engine.open(path: path);
      widget.store.remember(path);
      _setCanvas(engine);
      setState(() => _engine = engine);
    } on ApiError catch (e) {
      final messenger = _messenger.currentState;
      messenger?.showSnackBar(
        SnackBar(
          content: Text(messenger.context.error(e)),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  /// Back to the start screen; the project is already saved.
  /// The shell disposes its engine once it has faded out.
  void _home() => setState(() => _engine = null);

  final _messenger = GlobalKey<ScaffoldMessengerState>();

  @override
  Widget build(BuildContext context) {
    final engine = _engine;
    return Tokens(
      dark: _dark,
      child: MaterialApp(
        title: 'Rustle3D',
        debugShowCheckedModeBanner: false,
        scaffoldMessengerKey: _messenger,
        theme: buildTheme(_dark),
        locale: Locale(_language),
        localizationsDelegates: const [
          L.delegate,
          ...GlobalMaterialLocalizations.delegates,
        ],
        supportedLocales: L.supportedLocales,
        home: AnimatedSwitcher(
          duration: Motion.normal,
          child: engine == null
              ? StartScreen(
                  key: const ValueKey('start'),
                  store: widget.store,
                  dark: _dark,
                  onToggleTheme: _toggleTheme,
                  language: _language,
                  onLanguage: _setLanguage,
                  onOpen: _open,
                )
              : Shell(
                  key: ObjectKey(engine),
                  engine: engine,
                  libraryDir: widget.store.libraryDir,
                  dark: _dark,
                  onToggleTheme: _toggleTheme,
                  language: _language,
                  onLanguage: _setLanguage,
                  onHome: _home,
                ),
        ),
      ),
    );
  }
}
