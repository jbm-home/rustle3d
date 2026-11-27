// Where projects and the user's libraries live, the recent projects,
// making a new project (from a template or blank), and renaming, deleting,
// exporting and importing whole projects.

import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

import 'rust/api/engine.dart';
import 'rust/api/library.dart';
import 'rust/api/projects.dart';

/// A project folder found in the projects folder.
class RecentProject {
  const RecentProject({
    required this.path,
    required this.name,
    required this.modified,
  });

  final String path;
  final String name;
  final DateTime modified;
}

/// The user's folders: projects (Documents/Rustle3D) and libraries (body
/// presets, in the app's support folder).
class ProjectStore {
  const ProjectStore({required this.projectsDir, required this.libraryDir});

  final String projectsDir;
  final String libraryDir;

  static Future<ProjectStore> open() async {
    final docs = await getApplicationDocumentsDirectory();
    final support = await getApplicationSupportDirectory();
    final projects = Directory(
      normalize('${docs.path}${Platform.pathSeparator}Rustle3D'),
    );
    await projects.create(recursive: true);
    return ProjectStore(projectsDir: projects.path, libraryDir: support.path);
  }

  /// [path] spelled one way only, so that one folder is one entry: on
  /// Windows, `/` and `\` both separate folders, and each path source
  /// (the system, the file dialog, the engine) mixes them its own way.
  static String normalize(String path) =>
      Platform.isWindows ? path.replaceAll('/', r'\') : path;

  File get _recentFile => File('$libraryDir/recent-projects.json');

  /// The paths of the projects opened, most recent first.
  List<String> _openedPaths() {
    try {
      final paths = (jsonDecode(_recentFile.readAsStringSync()) as List)
          .cast<String>()
          .map(normalize);
      return {...paths}.toList();
    } on Object {
      return const [];
    }
  }

  /// Remember that the project at [path] was opened.
  void remember(String path) {
    final key = normalize(path);
    _writeOpened([key, ..._openedPaths().where((p) => p != key)]);
  }

  void _writeOpened(List<String> paths) {
    try {
      _recentFile
        ..createSync(recursive: true)
        ..writeAsStringSync(jsonEncode(paths.take(50).toList()));
    } on FileSystemException {
      // Not remembered: it is still listed if it is in the projects folder.
    }
  }

  /// Rename the project at [path] (closed) to [name]; returns its folder,
  /// renamed to match when that name is free.
  String rename(String path, String name) {
    final key = normalize(path);
    final renamed = normalize(renameProject(path: path, name: name));
    _writeOpened([for (final p in _openedPaths()) p == key ? renamed : p]);
    return renamed;
  }

  /// Delete the project at [path] (closed) for good.
  void delete(String path) {
    deleteProject(path: path);
    final key = normalize(path);
    _writeOpened(_openedPaths().where((p) => p != key).toList());
  }

  /// Pack the project at [path] into the zip archive [archive].
  Future<void> exportZip(String path, String archive) =>
      exportProjectZip(path: path, archive: archive);

  /// Unpack the project in the zip archive [archive] into the projects
  /// folder; returns its folder.
  Future<String> importZip(String archive) =>
      importProjectZip(archive: archive, parent: projectsDir);

  /// The project in the folder at [path], if it reads as one.
  static RecentProject? read(String path) {
    final file = File('$path/project.json');
    try {
      final json = jsonDecode(file.readAsStringSync());
      return RecentProject(
        path: path,
        name: json['project']['name'] as String,
        modified: file.lastModifiedSync(),
      );
    } on Object {
      // Missing or not readable as a project: not listed (opening it
      // would say why).
      return null;
    }
  }

  /// Every project: those in the projects folder and those opened from
  /// elsewhere, most recently changed first.
  List<RecentProject> all() {
    final paths = {..._openedPaths()};
    final dir = Directory(projectsDir);
    if (dir.existsSync()) {
      for (final entry in dir.listSync()) {
        if (entry is Directory && entry.path.endsWith('.r3d')) {
          paths.add(normalize(entry.path));
        }
      }
    }
    final found = [for (final path in paths) ?read(path)];
    found.sort((a, b) => b.modified.compareTo(a.modified));
    return found;
  }

  /// The standard body new projects start with.
  Map<String, dynamic> standardBody() {
    final presets = jsonDecode(bodyPresets(libraryDir: libraryDir)) as List;
    return presets.firstWhere((p) => p['name'] == 'Women M')
        as Map<String, dynamic>;
  }

  /// Make a project called [name] for [body] (a preset entry: `{name,
  /// body}`), or an object stuffed by [stuffing] when [body] is null
  /// (docs/specs/36-objects.md), drafted from [template] with [sizes] if
  /// given, its pieces renamed by [pieceNames], and return its folder.
  /// The project is then opened fresh, so its history starts empty.
  /// Drafting takes seconds (the body is fitted): [busy] runs the long
  /// steps, showing their progress.
  Future<String> create({
    required String name,
    required Map<String, dynamic>? body,
    double? stuffing,
    Future<String> Function(Engine engine, Future<String> Function()) busy =
        _noCard,
    String? template,
    Map<String, double> sizes = const {},
    Map<String, String> pieceNames = const {},
  }) async {
    final path = freeProjectPath(parent: projectsDir, name: name);
    final engine = Engine.open(path: path);
    try {
      engine.run(
        stepJson: jsonEncode(
          body == null
              ? {'type': 'object.set', 'stuffing': ?stuffing}
              : {
                  'type': 'avatar.set',
                  'body': body['body'],
                  'preset': ?body['name'],
                },
        ),
      );
      if (template != null) {
        await busy(
          engine,
          () => engine.runLong(
            stepJson: jsonEncode({
              'type': 'template.apply',
              'template': template,
              'sizes': sizes,
            }),
          ),
        );
        // The template names its pieces in English: give them their names
        // in the UI language.
        final summary = jsonDecode(
          engine.run(stepJson: jsonEncode({'type': 'project.summary'})),
        );
        for (final panel in (summary['panels'] as List? ?? const [])) {
          final name = pieceNames[panel['name']];
          if (name == null || name == panel['name']) continue;
          engine.run(
            stepJson: jsonEncode({
              'type': 'panel.rename',
              'panel': panel['id'],
              'name': name,
            }),
          );
        }
      }
      engine.run(
        stepJson: jsonEncode({'type': 'project.rename', 'name': name}),
      );
    } catch (_) {
      // Stopped or failed: leave no half-made project behind.
      engine.dispose();
      Directory(path).deleteSync(recursive: true);
      rethrow;
    }
    engine.dispose();
    return path;
  }
}

/// Long steps run as they are, without showing their progress (tests,
/// review aids).
Future<String> _noCard(Engine _, Future<String> Function() job) => job();
