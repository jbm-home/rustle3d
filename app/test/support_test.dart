// The donation reminder: when the start screen's card shows.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:rustle3d/src/support.dart';

void main() {
  late Directory dir;
  setUp(() => dir = Directory.systemTemp.createTempSync('r3d-support'));
  tearDown(() => dir.deleteSync(recursive: true));

  /// Launch [n] times on [day]; whether the last launch shows the card.
  bool launch(int n, DateTime day, {bool hasProjects = true}) {
    var shown = false;
    for (var i = 0; i < n; i++) {
      final memory = SupportMemory(dir.path)
        ..launched(hasProjects: hasProjects, now: day);
      shown = memory.remindNow;
    }
    return shown;
  }

  final day = DateTime(2027, 1, 1);

  test('not before ten launches, then once a quarter', () {
    expect(launch(9, day), isFalse);
    expect(launch(1, day), isTrue);
    expect(launch(1, day.add(const Duration(days: 89))), isFalse);
    expect(launch(1, day.add(const Duration(days: 90))), isTrue);
  });

  test('not without a project', () {
    expect(launch(12, day, hasProjects: false), isFalse);
    expect(launch(1, day), isTrue);
  });

  test('never again once turned off', () {
    launch(9, day);
    final memory = SupportMemory(dir.path)
      ..launched(hasProjects: true, now: day);
    expect(memory.remindNow, isTrue);
    memory.turnOff();
    expect(memory.remindNow, isFalse);
    expect(launch(1, day.add(const Duration(days: 400))), isFalse);
  });

  test('an unreadable file starts over', () {
    File('${dir.path}/support.json').writeAsStringSync('not json');
    expect(launch(1, day), isFalse);
    expect(launch(9, day), isTrue);
  });
}
