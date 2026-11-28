// The projector's calibration: the square measured on the table rescales
// each axis, and the settings survive in the library folder.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:rustle3d/src/projector.dart';

void main() {
  test('a square measured too large shrinks the scale', () {
    const s = ProjectorSettings();
    final c = s.calibrated(200, 220, 180);
    expect(c.pxPerMmX, closeTo(defaultPxPerMm * 200 / 220, 1e-12));
    expect(c.pxPerMmY, closeTo(defaultPxPerMm * 200 / 180, 1e-12));
  });

  test('settings are remembered, bad files ignored', () {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final memory = ProjectorMemory(dir);
    expect(memory.read().pxPerMmX, defaultPxPerMm);
    memory.write(
      const ProjectorSettings(
        pxPerMmX: 4,
        pxPerMmY: 4.2,
        mirror: true,
        quarterTurns: 3,
        dark: false,
      ),
    );
    final back = memory.read();
    expect(back.pxPerMmY, 4.2);
    expect(back.mirror, isTrue);
    expect(back.quarterTurns, 3);
    expect(back.dark, isFalse);
    File('$dir/projector.json').writeAsStringSync('{"px_per_mm_x": -1}');
    expect(memory.read().pxPerMmX, defaultPxPerMm);
    File('$dir/projector.json').writeAsStringSync('not json');
    expect(memory.read().mirror, isFalse);
  });
}
