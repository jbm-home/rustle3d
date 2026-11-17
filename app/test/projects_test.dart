import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:rustle3d/src/projects.dart';

void main() {
  late Directory root;
  late ProjectStore store;

  setUp(() {
    root = Directory.systemTemp.createTempSync('r3d-projects-');
    final projects = Directory('${root.path}${Platform.pathSeparator}projects')
      ..createSync();
    store = ProjectStore(
      projectsDir: ProjectStore.normalize(projects.path),
      libraryDir: root.path,
    );
  });

  tearDown(() => root.deleteSync(recursive: true));

  test('a project remembered under another spelling is listed once', () {
    final sep = Platform.pathSeparator;
    final path = '${store.projectsDir}${sep}Untitled.r3d';
    Directory(path).createSync();
    File(
      '$path${sep}project.json',
    ).writeAsStringSync('{"project": {"name": "Untitled"}}');

    store.remember(path.replaceAll(r'\', '/'));
    store.remember(path);

    expect(store.all().map((p) => p.name), ['Untitled']);
  });
}
