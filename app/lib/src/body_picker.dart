// Choosing the body of a new project (docs/specs/37-body-at-creation.md):
// who it dresses, a size of their chart, a silhouette and the height; the
// four measurements follow and can be typed, the size they make is shown
// as they change. Or one of the user's own bodies.

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'rust/api/engine.dart';
import 'rust/api/library.dart';
import 'rust/api/templates.dart';

/// The size a chart starts on: the size patterns are commonly drafted in.
const _startSize = {
  'women': '38',
  'men': '48',
  'girls': '6',
  'boys': '8',
  'babies': '12m',
};

/// The four measurements typed here, with the ranges `avatar.set` takes.
const _fields = {
  'height_cm': (50.0, 220.0),
  'bust_cm': (35.0, 170.0),
  'waist_cm': (33.0, 170.0),
  'hips_cm': (35.0, 180.0),
};

/// A chart's name for the UI.
String chartName(L l, String chart) => switch (chart) {
  'women' => l.chartWomen,
  'men' => l.chartMen,
  'girls' => l.chartGirls,
  'boys' => l.chartBoys,
  'babies' => l.chartBabies,
  _ => chart,
};

/// A size's name for the UI: the number for adults, the age for
/// children.
String sizeName(L l, String chart, String size) => switch (chart) {
  'girls' || 'boys' => l.sizeYears(size),
  'babies' => l.sizeMonths(size.replaceAll('m', '')),
  _ => size,
};

/// A silhouette's name and what it means, on [chart].
(String, String) _silhouetteText(L l, String chart, String s) {
  final child = chart != 'women' && chart != 'men';
  return switch (s) {
    'balanced' => (l.silhouetteBalanced, l.silhouetteBalancedHint),
    'hourglass' => (l.silhouetteHourglass, l.silhouetteHourglassHint),
    'pear' => (l.silhouettePear, l.silhouettePearHint),
    'straight' => (l.silhouetteStraight, l.silhouetteStraightHint),
    'round' when child => (l.silhouetteSturdy, l.silhouetteSturdyHint),
    'round' => (l.silhouetteRound, l.silhouetteRoundHint),
    'inverted_triangle' => (l.silhouetteInverted, l.silhouetteInvertedHint),
    'slim' => (l.silhouetteSlim, l.silhouetteSlimHint),
    _ => (s, ''),
  };
}

/// The chart and size of a built-in preset name ("Girls 6", "Baby 12m",
/// "Women 38"), if it is one of a chart's sizes.
(String, String)? chartSizeOf(String? preset) => switch (preset?.split(' ')) {
  ['Women', final s] => ('women', s),
  ['Men', final s] => ('men', s),
  ['Girls', final s] => ('girls', s),
  ['Boys', final s] => ('boys', s),
  ['Baby', final s] => ('babies', s),
  _ => null,
};

/// The preset name a chart size is recorded under (`avatar.set`'s
/// `preset`), translated like the built-in presets.
String _presetLabel(String chart, String size) => switch (chart) {
  'women' => 'Women $size',
  'men' => 'Men $size',
  'girls' => 'Girls $size',
  'boys' => 'Boys $size',
  _ => 'Baby $size',
};

class BodyPicker extends StatefulWidget {
  const BodyPicker({
    super.key,
    required this.libraryDir,
    required this.onChanged,
    this.start,
  });

  final String libraryDir;

  /// The preset a template starts on ("Girls 6"); women's 38 otherwise.
  final String? start;

  /// The body chosen, as a preset entry `{name, body}` (`name` null for
  /// typed measurements); null while a measurement is out of range.
  final ValueChanged<Map<String, dynamic>?> onChanged;

  @override
  State<BodyPicker> createState() => _BodyPickerState();
}

class _BodyPickerState extends State<BodyPicker> {
  late final List<Map<String, dynamic>> _charts =
      (jsonDecode(sizeCharts()) as List).cast<Map<String, dynamic>>();
  late final List<Map<String, dynamic>> _mine =
      (jsonDecode(bodyPresets(libraryDir: widget.libraryDir)) as List)
          .cast<Map<String, dynamic>>()
          .where((p) => p['builtin'] == false)
          .toList();
  final _values = {for (final k in _fields.keys) k: TextEditingController()};

  late String _chart;
  late String _size;
  String _silhouette = 'balanced';

  /// The measurements were typed (or come from one of the user's
  /// bodies), not the chart's.
  bool _typed = false;

  /// The user's body they come from, if any.
  String? _mineName;

  /// The size the measurements make, on the chart that dresses them.
  String? _nearest;
  String? _error;

  @override
  void initState() {
    super.initState();
    final (chart, size) = chartSizeOf(widget.start) ?? ('women', '38');
    _chart = chart;
    _size = _sizes(chart).contains(size) ? size : _startSize[chart]!;
    _fill(keepHeight: false);
    WidgetsBinding.instance.addPostFrameCallback((_) => _emit());
  }

  @override
  void dispose() {
    for (final c in _values.values) {
      c.dispose();
    }
    super.dispose();
  }

  Map<String, dynamic> get _chartEntry =>
      _charts.firstWhere((c) => c['chart'] == _chart);

  List<String> _sizes(String chart) => [
    for (final c in _charts)
      if (c['chart'] == chart)
        for (final s in c['sizes'] as List) s['name'] as String,
  ];

  /// The masculinity and age the chart gives its bodies.
  Map<String, dynamic> get _chartBody =>
      ((_chartEntry['sizes'] as List).firstWhere((s) => s['name'] == _size)
              as Map)['body']
          as Map<String, dynamic>;

  String _text(num v) => v % 1 == 0 ? v.toStringAsFixed(0) : '$v';

  /// The chart's measurements for the size and silhouette; the height
  /// typed is kept unless [keepHeight] is false.
  void _fill({bool keepHeight = true}) {
    final height = keepHeight
        ? double.tryParse(_values['height_cm']!.text.replaceAll(',', '.'))
        : null;
    final body = jsonDecode(
      sizeBody(
        chart: _chart,
        size: _size,
        silhouette: _silhouette,
        heightCm: height,
      ),
    ) as Map<String, dynamic>;
    for (final e in _values.entries) {
      e.value.text = _text(body[e.key] as num);
    }
    _typed = false;
    _mineName = null;
  }

  /// The body the fields make, or null with [_error] set.
  Map<String, dynamic>? _read() {
    final base = _mineName != null
        ? _mine.firstWhere((p) => p['name'] == _mineName)['body'] as Map
        : _chartBody;
    final body = <String, dynamic>{
      'masculinity': base['masculinity'],
      if (base['age_years'] != null) 'age_years': base['age_years'],
    };
    for (final e in _values.entries) {
      final v = double.tryParse(e.value.text.replaceAll(',', '.'));
      final (min, max) = _fields[e.key]!;
      if (v == null || v < min || v > max) {
        _error = context.l.measBetween(
          _fieldLabel(context.l, e.key),
          min.round(),
          max.round(),
        );
        return null;
      }
      body[e.key] = v;
    }
    _error = null;
    return body;
  }

  void _emit() {
    if (!mounted) return;
    final body = _read();
    String? nearest;
    if (body != null) {
      final got = jsonDecode(nearestSize(bodyJson: jsonEncode(body)));
      nearest = got['chart'] == _chart ? got['name'] as String? : null;
    }
    setState(() => _nearest = nearest);
    widget.onChanged(
      body == null
          ? null
          : {
              'name':
                  _mineName ?? (_typed ? null : _presetLabel(_chart, _size)),
              'body': body,
            },
    );
  }

  void _chooseChart(String chart) {
    setState(() {
      _chart = chart;
      _size = _startSize[chart]!;
      _silhouette = 'balanced';
      _fill(keepHeight: false);
    });
    _emit();
  }

  void _chooseSize(String size) {
    setState(() {
      _size = size;
      // A child's height is its age: it follows the size.
      _fill(keepHeight: _chart == 'women' || _chart == 'men');
    });
    _emit();
  }

  void _chooseSilhouette(String s) {
    setState(() {
      _silhouette = s;
      _fill();
    });
    _emit();
  }

  void _chooseMine(Map<String, dynamic> preset) {
    final body = preset['body'] as Map<String, dynamic>;
    final chart = jsonDecode(nearestSize(bodyJson: jsonEncode(body)));
    setState(() {
      _chart = chart['chart'] as String;
      _size = chart['name'] as String? ?? _startSize[_chart]!;
      for (final e in _values.entries) {
        e.value.text = _text(body[e.key] as num);
      }
      _typed = true;
      _mineName = preset['name'] as String;
    });
    _emit();
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final silhouettes = (_chartEntry['silhouettes'] as List).cast<String>();
    final differs = _typed && _nearest != null && _nearest != _size;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            Expanded(
              child: SegmentedButton<String>(
                key: const Key('bodyChart'),
                showSelectedIcon: false,
                segments: [
                  for (final c in _charts)
                    ButtonSegment(
                      value: c['chart'] as String,
                      label: Text(chartName(l, c['chart'] as String)),
                    ),
                ],
                selected: {_chart},
                onSelectionChanged: (s) => _chooseChart(s.first),
              ),
            ),
            if (_mine.isNotEmpty) ...[
              const SizedBox(width: Space.s),
              PopupMenuButton<Map<String, dynamic>>(
                tooltip: l.myBodiesTip,
                onSelected: _chooseMine,
                itemBuilder: (_) => [
                  for (final x in _mine)
                    PopupMenuItem(value: x, child: Text(x['name'] as String)),
                ],
                child: Padding(
                  padding: const EdgeInsets.all(Space.s),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.person_outline_rounded,
                        size: 18,
                        color: p.accent,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        _mineName ?? l.myBodies,
                        style: ts(fontSize: 13, color: p.accent),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ],
        ),
        const SizedBox(height: Space.l),
        Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            SizedBox(
              width: 150,
              child: DropdownButtonFormField<String>(
                key: const Key('bodySize'),
                initialValue: _size,
                isDense: true,
                decoration: InputDecoration(
                  labelText: _chart == 'women' || _chart == 'men'
                      ? l.bodySize
                      : l.bodyAge,
                  border: const OutlineInputBorder(),
                ),
                items: [
                  for (final s in _sizes(_chart))
                    DropdownMenuItem(
                      value: s,
                      child: Text(sizeName(l, _chart, s)),
                    ),
                ],
                onChanged: (s) {
                  if (s != null) _chooseSize(s);
                },
              ),
            ),
            const SizedBox(width: Space.m),
            Expanded(
              child: AnimatedSwitcher(
                duration: const Duration(milliseconds: 150),
                child: Text(
                  key: ValueKey((_typed, _nearest)),
                  differs
                      ? l.typedNearestSize(sizeName(l, _chart, _nearest!))
                      : _typed
                      ? l.typedMeasurements
                      : l.chartMeasurements(chartName(l, _chart)),
                  style: ts(
                    fontSize: 12.5,
                    color: differs ? p.accent : p.textMuted,
                    height: 1.35,
                  ),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: Space.m),
        Text(
          l.silhouette,
          style: ts(fontSize: 13, fontWeight: FontWeight.w600, color: p.text),
        ),
        const SizedBox(height: Space.xs),
        Wrap(
          spacing: Space.s,
          runSpacing: Space.s,
          children: [
            for (final s in silhouettes)
              Tooltip(
                message: _silhouetteText(l, _chart, s).$2,
                child: ChoiceChip(
                  key: Key('silhouette_$s'),
                  label: Text(_silhouetteText(l, _chart, s).$1),
                  selected: !_typed && _silhouette == s,
                  onSelected: (_) => _chooseSilhouette(s),
                ),
              ),
          ],
        ),
        const SizedBox(height: Space.l),
        Wrap(
          spacing: Space.m,
          runSpacing: Space.m,
          children: [
            for (final e in _values.entries)
              SizedBox(
                width: 108,
                child: TextField(
                  key: Key('bodyField_${e.key}'),
                  controller: e.value,
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                  ),
                  inputFormatters: [
                    FilteringTextInputFormatter.allow(RegExp(r'[0-9.,]')),
                  ],
                  onChanged: (_) {
                    _typed = true;
                    _emit();
                  },
                  decoration: InputDecoration(
                    labelText: _fieldLabel(l, e.key),
                    suffixText: l.cm,
                    isDense: true,
                    border: const OutlineInputBorder(),
                  ),
                ),
              ),
          ],
        ),
        if (_error != null) ...[
          const SizedBox(height: Space.s),
          Text(_error!, style: ts(fontSize: 13, color: p.danger)),
        ],
      ],
    );
  }
}

String _fieldLabel(L l, String key) => switch (key) {
  'height_cm' => l.measHeight,
  'bust_cm' => l.measBust,
  'waist_cm' => l.sizeWaist,
  'hips_cm' => l.sizeHips,
  _ => key,
};

/// The size `body` (a `Body` as JSON) makes on the chart that dresses it,
/// for display: "40 (Woman)".
String? nearestSizeText(L l, Map<String, dynamic> body) {
  try {
    final got = jsonDecode(nearestSize(bodyJson: jsonEncode(body)));
    final name = got['name'] as String?;
    if (name == null) return null;
    final chart = got['chart'] as String;
    return l.nearestSizeOn(sizeName(l, chart, name), chartName(l, chart));
  } on ApiError {
    return null;
  }
}
