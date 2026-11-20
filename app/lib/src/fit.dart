// Fit maps and the tape measure over the 3D view
// (docs/specs/24-fit-maps-tape.md): the menu that colours the cloth by
// strain or by its gap to the body, the map's legend, and the reading of
// the tape measure taken on the drape.

part of 'shell.dart';

/// Off, Strain or Gap: what the cloth in the 3D view is coloured by.
class _FitButton extends StatelessWidget {
  const _FitButton({required this.map, required this.onMap});

  /// `strain`, `gap`, or `null` for the fabric.
  final String? map;
  final ValueChanged<String?> onMap;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final choices = [
      (null, l.fitOff),
      ('strain', l.fitStrain),
      ('gap', l.fitGap),
    ];
    // Menu values cannot be null: '' stands for the fabric.
    return _Pill(
      child: PopupMenuButton<String>(
        key: const ValueKey('fitButton'),
        tooltip: l.fitTip,
        onSelected: (choice) => onMap(choice.isEmpty ? null : choice),
        itemBuilder: (_) => [
          for (final (value, label) in choices)
            CheckedPopupMenuItem(
              value: value ?? '',
              checked: value == map,
              child: Text(label),
            ),
        ],
        icon: Icon(
          Icons.thermostat_rounded,
          size: 19,
          color: map == null ? p.textMuted : p.accent,
        ),
      ),
    );
  }
}

/// The fit map's scale: a bar from loose to tight with its two ends, what
/// the map shows, and the tightest value on the cloth now.
class _FitLegendCard extends StatelessWidget {
  const _FitLegendCard({required this.legend});

  final FitLegend legend;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final strain = legend.map == 'strain';
    String value(double v) =>
        '${context.number(v, strain ? 1 : 0)} ${legend.unit}';
    // The loose end is open-ended for the gap (50 mm and more), the tight
    // one for the strain (20 % and more).
    final loose = strain ? value(legend.loose) : '${value(legend.loose)}+';
    final tight = strain ? '${value(legend.tight)}+' : value(legend.tight);
    final small = ts(fontSize: 11.5, color: p.textMuted);
    return Container(
      key: const ValueKey('fitLegend'),
      width: 220,
      padding: const EdgeInsets.all(Space.m),
      decoration: BoxDecoration(
        color: p.surface.withValues(alpha: 0.92),
        borderRadius: BorderRadius.circular(Tokens.radius(context, 0.75)),
        border: Border.all(color: p.border),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            strain ? l.fitStrainTitle : l.fitGapTitle,
            style: ts(fontSize: 12.5, fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: Space.s),
          Container(
            height: 10,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(3),
              gradient: LinearGradient(
                colors: [for (final c in legend.colors) Color(c)],
              ),
            ),
          ),
          const SizedBox(height: Space.xs),
          Row(
            children: [
              Text('$loose · ${l.fitLoose}', style: small),
              const Spacer(),
              Text('${l.fitTight} · $tight', style: small),
            ],
          ),
          if (legend.tightest case final v?) ...[
            const SizedBox(height: Space.s),
            Text(
              l.fitTightest(value(v), legend.tightestPiece ?? ''),
              style: ts(fontSize: 12),
            ),
          ],
        ],
      ),
    );
  }
}

/// The tape measure's card. Round: the height, the garment's and the
/// body's circumferences, the ease and the gap at the point clicked.
/// Length: between two points clicked on the cloth, along it, by tape
/// and straight.
class _TapeCard extends StatelessWidget {
  const _TapeCard({
    required this.length,
    required this.onLength,
    required this.reading,
    required this.started,
    required this.lengthReading,
    required this.onClose,
  });

  /// Measuring a length between two points, not round.
  final bool length;
  final ValueChanged<bool> onLength;

  /// Round: `null` before the first click.
  final TapeReading? reading;

  /// Length: the first point is clicked, the second awaited.
  final bool started;

  /// Length: `null` until both points are clicked.
  final LengthReading? lengthReading;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    String cm(double? v, [bool signed = false]) {
      if (v == null) return '–';
      final text = '${context.number(v, 1)} cm';
      return signed && v > 0 ? '+$text' : text;
    }

    Widget row(String label, String value, {bool strong = false}) => Padding(
      padding: const EdgeInsets.only(top: 3),
      child: Row(
        children: [
          Text(label, style: ts(fontSize: 12, color: p.textMuted)),
          const Spacer(),
          Text(
            value,
            style: ts(
              fontSize: 12.5,
              fontWeight: strong ? FontWeight.w600 : FontWeight.w400,
            ),
          ),
        ],
      ),
    );

    Widget note(String text) => Padding(
      padding: const EdgeInsets.only(top: Space.xs),
      child: Text(text, style: ts(fontSize: 11.5, color: p.textMuted)),
    );

    Widget hint(String text) =>
        Text(text, style: ts(fontSize: 12, color: p.textMuted));

    final List<Widget> body;
    if (length) {
      final r = lengthReading;
      body = [
        if (r == null)
          hint(started ? l.tapeSecondPoint : l.tapeFirstPoint)
        else ...[
          row(l.tapeAlong, cm(r.alongCm), strong: true),
          row(l.tapeTaut, cm(r.tapeCm)),
          row(l.tapeStraight, cm(r.straightCm)),
          if (r.alongCm == null) note(l.tapeNotJoined),
        ],
      ];
    } else {
      final r = reading;
      body = [
        if (r == null)
          hint(l.tapeHint)
        else ...[
          row(l.tapeHeight, cm(r.heightCm)),
          row(l.tapeGarment, cm(r.garmentCm)),
          row(l.tapeBody, cm(r.bodyCm)),
          row(l.tapeEase, cm(r.easeCm, true), strong: true),
          row(
            l.tapeGap,
            r.gapMm == null ? '–' : '${context.number(r.gapMm!)} mm',
          ),
          if (!r.closed && r.garmentCm != null) note(l.tapeOpen),
        ],
      ];
    }

    return Container(
      key: const ValueKey('tapeCard'),
      width: 230,
      padding: const EdgeInsets.fromLTRB(Space.m, Space.s, Space.xs, Space.m),
      decoration: BoxDecoration(
        color: p.surface.withValues(alpha: 0.94),
        borderRadius: BorderRadius.circular(Tokens.radius(context, 0.75)),
        border: Border.all(color: p.border),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.straighten_rounded, size: 16, color: p.accent),
              const SizedBox(width: Space.s),
              Text(
                l.toolTape,
                style: ts(fontSize: 12.5, fontWeight: FontWeight.w600),
              ),
              const Spacer(),
              _IconAction(
                icon: Icons.close_rounded,
                tip: l.tapeClose,
                onTap: onClose,
              ),
            ],
          ),
          Padding(
            padding: const EdgeInsets.only(right: Space.s),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: Space.xs),
                SegmentedButton<bool>(
                  key: const ValueKey('tapeMode'),
                  showSelectedIcon: false,
                  style: const ButtonStyle(
                    visualDensity: VisualDensity.compact,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                  segments: [
                    ButtonSegment(value: false, label: Text(l.tapeRound)),
                    ButtonSegment(value: true, label: Text(l.tapeLength)),
                  ],
                  selected: {length},
                  onSelectionChanged: (s) => onLength(s.first),
                ),
                const SizedBox(height: Space.s),
                ...body,
              ],
            ),
          ),
        ],
      ),
    );
  }
}
