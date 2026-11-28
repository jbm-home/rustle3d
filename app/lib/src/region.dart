// How sizes and lengths are written for the user (docs/specs/38-region.md):
// a size system (EU 38, US 6, UK 10 or letters) and units (centimetres or
// inches). Guessed from the system's region the first time, then the
// user's own choice, remembered in their library folder.

import 'dart:convert';
import 'dart:io';

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'rust/api/engine.dart';
import 'rust/api/templates.dart';

enum SizeSystem { eu, us, uk, letters }

enum Units { metric, imperial }

/// Millimetres in an inch.
const mmPerInch = 25.4;

/// The user's size system and units.
@immutable
class Region {
  const Region({this.sizes = SizeSystem.eu, this.units = Units.metric});

  final SizeSystem sizes;
  final Units units;

  /// The region of a locale name ("fr_FR", "en-US"): US sizes and inches
  /// in the United States, UK sizes in the UK, Ireland, Australia and New
  /// Zealand, EU sizes and centimetres elsewhere.
  factory Region.guess(String localeName) {
    final country = localeName
        .split('.')
        .first
        .split(RegExp('[_-]'))
        .skip(1)
        .firstWhere((p) => p.length == 2, orElse: () => '')
        .toUpperCase();
    return switch (country) {
      'US' ||
      'LR' ||
      'MM' => const Region(sizes: SizeSystem.us, units: Units.imperial),
      'CA' => const Region(sizes: SizeSystem.us),
      'GB' || 'IE' || 'AU' || 'NZ' => const Region(sizes: SizeSystem.uk),
      _ => const Region(),
    };
  }

  Region copyWith({SizeSystem? sizes, Units? units}) =>
      Region(sizes: sizes ?? this.sizes, units: units ?? this.units);

  bool get inches => units == Units.imperial;

  /// A length in centimetres as shown: centimetres or inches.
  double fromCm(double cm) => inches ? cm * 10 / mmPerInch : cm;

  /// A length as typed back to centimetres.
  double toCm(double shown) => inches ? shown * mmPerInch / 10 : shown;

  /// The unit lengths in centimetres are shown in.
  String cmUnit(L l) => inches ? l.inch : l.cm;

  /// The decimals a body measurement is shown with.
  int get bodyDigits => inches ? 1 : 0;

  @override
  bool operator ==(Object other) =>
      other is Region && other.sizes == sizes && other.units == units;

  @override
  int get hashCode => Object.hash(sizes, units);
}

/// The region chosen, remembered in the user's library folder.
class RegionMemory {
  const RegionMemory(this.libraryDir);

  final String libraryDir;

  File get _file => File('$libraryDir/region.json');

  /// The saved region; guessed from the system's locale if none.
  Region read() {
    try {
      final json = jsonDecode(_file.readAsStringSync());
      if (json is Map<String, Object?>) {
        return Region(
          sizes: SizeSystem.values.asNameMap()[json['sizes']] ?? SizeSystem.eu,
          units: Units.values.asNameMap()[json['units']] ?? Units.metric,
        );
      }
    } on FileSystemException {
      // Not chosen yet.
    } on FormatException {
      // Unreadable: guessed again.
    }
    return Region.guess(Platform.localeName);
  }

  void write(Region region) {
    try {
      _file
        ..createSync(recursive: true)
        ..writeAsStringSync(
          jsonEncode({'sizes': region.sizes.name, 'units': region.units.name}),
        );
    } on FileSystemException {
      // Not remembered: guessed again next time, nothing worse.
    }
  }
}

/// Gives the region to the widgets under it; they rebuild when it
/// changes.
class RegionScope extends InheritedNotifier<ValueNotifier<Region>> {
  const RegionScope({
    super.key,
    required ValueNotifier<Region> region,
    required this.memory,
    required super.child,
  }) : super(notifier: region);

  final RegionMemory? memory;
}

extension RegionOf on BuildContext {
  /// The user's region (EU sizes and centimetres outside a scope).
  Region get region =>
      dependOnInheritedWidgetOfExactType<RegionScope>()?.notifier?.value ??
      const Region();

  /// Change the region everywhere, and remember it.
  void setRegion(Region region) {
    final scope = getInheritedWidgetOfExactType<RegionScope>();
    scope?.memory?.write(region);
    scope?.notifier?.value = region;
  }
}

/// Every adult size's names in the other systems, by chart and EU name
/// (`grading.charts`'s `labels`).
final Map<String, Map<String, Map<String, String>>> _labels = () {
  try {
    return {
      for (final c in jsonDecode(sizeCharts()) as List)
        c['chart'] as String: {
          for (final s in c['sizes'] as List)
            if (s['labels'] case final Map labels)
              s['name'] as String: labels.cast<String, String>(),
        },
    };
  } on ApiError {
    return const <String, Map<String, Map<String, String>>>{};
  }
}();

/// A size's name for the UI: in the user's system for adults, the age
/// for children.
String sizeName(BuildContext context, String chart, String size) {
  final l = context.l;
  return switch (chart) {
    'girls' || 'boys' => l.sizeYears(size),
    'babies' => l.sizeMonths(size.replaceAll('m', '')),
    _ => _labels[chart]?[size]?[context.region.sizes.name] ?? size,
  };
}

/// What a size is in the other systems, to tell sizes apart: "EU 38 · M"
/// beside a US 6, "M" beside an EU 38, "EU 38" beside an M; none for
/// children.
String? sizeAside(BuildContext context, String chart, String size) {
  final labels = _labels[chart]?[size];
  if (labels == null) return null;
  final eu = '${context.l.sizeSystemEu} $size';
  return switch (context.region.sizes) {
    SizeSystem.eu => labels['letters'],
    SizeSystem.letters => eu,
    _ => '$eu · ${labels['letters']}',
  };
}

/// A size in a menu: its name, and what it is in other systems in grey.
Widget sizeItem(BuildContext context, String chart, String size) {
  final aside = sizeAside(context, chart, size);
  return Text.rich(
    TextSpan(
      text: sizeName(context, chart, size),
      children: [
        if (aside != null)
          TextSpan(
            text: '   $aside',
            style: ts(fontSize: 12, color: Tokens.of(context).textMuted),
          ),
      ],
    ),
  );
}

/// The size system's short name: "EU", "US", "UK", "S·M·L".
String sizeSystemName(L l, SizeSystem s) => switch (s) {
  SizeSystem.eu => l.sizeSystemEu,
  SizeSystem.us => l.sizeSystemUs,
  SizeSystem.uk => l.sizeSystemUk,
  SizeSystem.letters => l.sizeSystemLetters,
};

/// A small button that shows the size system and changes it.
class SizeSystemButton extends StatelessWidget {
  const SizeSystemButton({super.key});

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    final p = Tokens.of(context);
    final region = context.region;
    return PopupMenuButton<SizeSystem>(
      key: const Key('sizeSystem'),
      tooltip: l.sizeSystemTip,
      initialValue: region.sizes,
      onSelected: (s) => context.setRegion(region.copyWith(sizes: s)),
      itemBuilder: (_) => [
        for (final s in SizeSystem.values)
          PopupMenuItem(
            key: Key('sizeSystem_${s.name}'),
            value: s,
            child: Text(
              '${sizeSystemName(l, s)}   ${switch (s) {
                SizeSystem.eu => '36 · 38 · 40',
                SizeSystem.us => '4 · 6 · 8',
                SizeSystem.uk => '8 · 10 · 12',
                SizeSystem.letters => 'S · M · L',
              }}',
            ),
          ),
      ],
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: Space.s, vertical: 6),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              sizeSystemName(l, region.sizes),
              style: ts(fontSize: 13, color: p.accent),
            ),
            Icon(Icons.arrow_drop_down_rounded, size: 18, color: p.accent),
          ],
        ),
      ),
    );
  }
}

/// A small cm | in switch for the units lengths are shown in.
class UnitsButton extends StatelessWidget {
  const UnitsButton({super.key});

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    final region = context.region;
    return Tooltip(
      message: l.unitsTip,
      child: SegmentedButton<Units>(
        key: const Key('units'),
        showSelectedIcon: false,
        style: const ButtonStyle(visualDensity: VisualDensity.compact),
        segments: [
          ButtonSegment(value: Units.metric, label: Text(l.cm)),
          ButtonSegment(value: Units.imperial, label: Text(l.inch)),
        ],
        selected: {region.units},
        onSelectionChanged: (s) =>
            context.setRegion(region.copyWith(units: s.first)),
      ),
    );
  }
}

/// A length in centimetres as typed in a field: whole centimetres (or
/// [digits] decimals), inches to one decimal.
String cmText(Region region, double cm, {int digits = 0}) {
  final v = region.fromCm(cm);
  final d = region.inches ? 1 : digits;
  final r = (v * _pow10(d)).round() / _pow10(d);
  return r == r.roundToDouble() ? r.toStringAsFixed(0) : r.toStringAsFixed(d);
}

double _pow10(int d) => [1.0, 10.0, 100.0, 1000.0][d.clamp(0, 3)];

/// A field's length back in centimetres; null if it is not a number.
double? cmOf(Region region, String text) {
  final v = double.tryParse(text.replaceAll(',', '.'));
  return v == null ? null : region.toCm(v);
}

/// Rewrites the fields typed in [from]'s units in [to]'s.
void convertCmFields(
  Iterable<TextEditingController> fields,
  Region from,
  Region to,
) {
  if (from.units == to.units) return;
  for (final c in fields) {
    if (cmOf(from, c.text) case final cm?) c.text = cmText(to, cm, digits: 1);
  }
}

/// What a number in a field or a label is, in the document's own unit:
/// a length in millimetres or centimetres (shown in inches in the imperial
/// units) or in metres (yards), an area in square centimetres (square
/// inches) or square metres (square yards), an angle, a percentage, a
/// count.
enum Quantity { mm, cm, m, cm2, m2, degrees, percent, count }

/// Metres in a yard.
const mPerYard = 0.9144;

/// Square centimetres in a square inch.
const _cm2PerIn2 = mmPerInch * mmPerInch / 100;

extension QuantityShown on Region {
  /// [q]'s unit for the user: mm, cm or in, °, %, none for a count.
  String unitOf(L l, Quantity q) => switch (q) {
    Quantity.mm => inches ? l.inch : l.mm,
    Quantity.cm => inches ? l.inch : l.cm,
    Quantity.m => inches ? l.yard : l.metre,
    Quantity.cm2 => '${inches ? l.inch : l.cm}²',
    Quantity.m2 => '${inches ? l.yard : l.metre}²',
    Quantity.degrees => '°',
    Quantity.percent => '%',
    Quantity.count => '',
  };

  /// A value of [q] as shown.
  double shown(Quantity q, double v) => switch (q) {
    Quantity.mm when inches => v / mmPerInch,
    Quantity.cm when inches => v * 10 / mmPerInch,
    Quantity.m when inches => v / mPerYard,
    Quantity.cm2 when inches => v / _cm2PerIn2,
    Quantity.m2 when inches => v / (mPerYard * mPerYard),
    _ => v,
  };

  /// A value as typed, back in [q]'s own unit.
  double typed(Quantity q, double shown) => switch (q) {
    Quantity.mm when inches => shown * mmPerInch,
    Quantity.cm when inches => shown * mmPerInch / 10,
    Quantity.m when inches => shown * mPerYard,
    Quantity.cm2 when inches => shown * _cm2PerIn2,
    Quantity.m2 when inches => shown * mPerYard * mPerYard,
    _ => shown,
  };

  /// The decimals [q] is shown with, [digits] being the metric ones: an
  /// inch is 2.54 cm, so one more for centimetres and two more for
  /// millimetres, at most two (a quarter of a millimetre).
  int digitsOf(Quantity q, int digits) => switch (q) {
    Quantity.mm when inches => (digits + 2).clamp(0, 2),
    Quantity.cm when inches => (digits + 1).clamp(0, 2),
    _ => digits,
  };

  /// [v] rounded to what is shown of it, in its own unit.
  double rounded(Quantity q, double v, int digits) {
    final f = _pow10(digitsOf(q, digits));
    return typed(q, (shown(q, v) * f).round() / f);
  }
}

extension QuantityOf on BuildContext {
  /// [q]'s unit in the user's units: mm, cm or in, °, %.
  String unitOf(Quantity q) => region.unitOf(l, q);

  /// [v] for a label, with its unit: "12.5 cm", "4.92 in" ([digits] in
  /// the metric units).
  String measure(Quantity q, num v, [int digits = 0]) {
    final r = region;
    return '${number(r.shown(q, v.toDouble()), r.digitsOf(q, digits))} '
        '${r.unitOf(l, q)}';
  }

  /// A length in millimetres for a label, to the millimetre: "12.5 cm",
  /// "4.92 in".
  String length(num mm) => measure(Quantity.cm, mm / 10, 1);

  /// [v] for a field the user edits, in the user's units.
  String fieldText(Quantity q, num v, [int digits = 0]) {
    final r = region;
    return decimal(r.shown(q, v.toDouble()), r.digitsOf(q, digits));
  }

  /// A field's number back in [q]'s own unit; null if it is not one.
  double? fieldValue(Quantity q, String text) {
    final v = double.tryParse(text.trim().replaceAll(',', '.'));
    return v == null ? null : region.typed(q, v);
  }
}
