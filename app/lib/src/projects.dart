// Where projects and the user's libraries live, the recent projects, and
// making a new project (from a template or blank).

import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

import 'rust/api/engine.dart';
import 'rust/api/library.dart';

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
    final projects = Directory('${docs.path}/Rustle3D');
    await projects.create(recursive: true);
    return ProjectStore(projectsDir: projects.path, libraryDir: support.path);
  }

  File get _recentFile => File('$libraryDir/recent-projects.json');

  /// The paths of the projects opened, most recent first.
  List<String> _openedPaths() {
    try {
      return (jsonDecode(_recentFile.readAsStringSync()) as List)
          .cast<String>();
    } on Object {
      return const [];
    }
  }

  /// Remember that the project at [path] was opened.
  void remember(String path) {
    final paths = [path, ..._openedPaths().where((p) => p != path)];
    try {
      _recentFile
        ..createSync(recursive: true)
        ..writeAsStringSync(jsonEncode(paths.take(50).toList()));
    } on FileSystemException {
      // Not remembered: it is still listed if it is in the projects folder.
    }
  }

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
          paths.add(entry.path);
        }
      }
    }
    final found = [for (final path in paths) ?read(path)];
    found.sort((a, b) => b.modified.compareTo(a.modified));
    return found;
  }

  /// A folder path for a new project called [name] that does not exist yet.
  String freePath(String name) {
    final safe = name.replaceAll(RegExp(r'[/\\:*?"<>|]'), ' ').trim();
    final stem = safe.isEmpty ? 'Untitled' : safe;
    var path = '$projectsDir/$stem.r3d';
    for (var n = 2; Directory(path).existsSync(); n++) {
      path = '$projectsDir/$stem $n.r3d';
    }
    return path;
  }

  /// The standard body new projects start with.
  Map<String, dynamic> standardBody() {
    final presets = jsonDecode(bodyPresets(libraryDir: libraryDir)) as List;
    return presets.firstWhere((p) => p['name'] == 'Women M')
        as Map<String, dynamic>;
  }

  /// Make a project called [name] for [body] (a preset entry: `{name,
  /// body}`), drafted from [template] with [sizes] if given, its pieces
  /// renamed by [pieceNames], and return its folder. The project is then opened fresh, so its history starts
  /// empty.
  String create({
    required String name,
    required Map<String, dynamic> body,
    String? template,
    Map<String, double> sizes = const {},
    Map<String, String> pieceNames = const {},
  }) {
    final path = freePath(name);
    final engine = Engine.open(path: path);
    try {
      engine.run(
        stepJson: jsonEncode({
          'type': 'avatar.set',
          'body': body['body'],
          'preset': body['name'],
        }),
      );
      if (template != null) {
        engine.run(
          stepJson: jsonEncode({
            'type': 'template.apply',
            'template': template,
            'sizes': sizes,
          }),
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
    } finally {
      engine.dispose();
    }
    return path;
  }
}
