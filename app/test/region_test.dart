// Lengths in the user's units (docs/specs/38-region.md): stored in mm or
// cm, shown and typed in inches.

import 'package:flutter_test/flutter_test.dart';
import 'package:rustle3d/src/region.dart';

void main() {
  const metric = Region();
  const imperial = Region(units: Units.imperial);

  test('metric lengths are shown as they are stored', () {
    for (final q in Quantity.values) {
      expect(metric.shown(q, 12.5), 12.5);
      expect(metric.typed(q, 12.5), 12.5);
      expect(metric.digitsOf(q, 1), 1);
    }
  });

  test('lengths and areas in inches and yards, and back', () {
    expect(imperial.shown(Quantity.mm, 25.4), closeTo(1, 1e-12));
    expect(imperial.shown(Quantity.cm, 2.54), closeTo(1, 1e-12));
    expect(imperial.shown(Quantity.m, 0.9144), closeTo(1, 1e-12));
    expect(imperial.shown(Quantity.cm2, 6.4516), closeTo(1, 1e-12));
    expect(imperial.shown(Quantity.m2, 0.83612736), closeTo(1, 1e-12));
    for (final q in Quantity.values) {
      expect(imperial.typed(q, imperial.shown(q, 7.3)), closeTo(7.3, 1e-12));
    }
    expect(imperial.shown(Quantity.degrees, 45), 45);
    expect(imperial.shown(Quantity.percent, 75), 75);
  });

  test('inches get the decimals of the metric precision, at most two', () {
    expect(imperial.digitsOf(Quantity.cm, 0), 1);
    expect(imperial.digitsOf(Quantity.cm, 1), 2);
    expect(imperial.digitsOf(Quantity.mm, 0), 2);
    expect(imperial.digitsOf(Quantity.mm, 1), 2);
    expect(imperial.digitsOf(Quantity.degrees, 0), 0);
  });

  test('rounded to what is shown', () {
    // A quarter inch stays a quarter inch: 6.35 mm, not 6.
    expect(imperial.rounded(Quantity.mm, 6.35, 0), closeTo(6.35, 1e-12));
    expect(imperial.rounded(Quantity.mm, 10, 0), closeTo(0.39 * 25.4, 1e-12));
    expect(metric.rounded(Quantity.mm, 6.35, 0), 6);
    expect(metric.rounded(Quantity.cm, 12.34, 1), closeTo(12.3, 1e-12));
  });
}
