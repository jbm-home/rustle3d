// Design tokens: the only place where colours, type, radii, spacing and
// motion are defined (docs/ux.md §3). The canvas colours are sent to the
// Rust renderer, so the board and the 3D view match the app.
//
// Style "Studio": cool neutrals, a violet accent, Manrope, crisp corners.

import 'dart:typed_data';

import 'package:material_ui/material_ui.dart';

import '../rust/api/engine.dart';

class Palette {
  const Palette({
    required this.background,
    required this.surface,
    required this.surfaceMuted,
    required this.border,
    required this.text,
    required this.textMuted,
    required this.accent,
    required this.accentSoft,
    required this.onAccent,
    required this.danger,
    required this.board,
    required this.gridMinor,
    required this.gridMajor,
    required this.line,
    required this.skyTop,
    required this.skyBottom,
    required this.ground,
    required this.cloth,
    required this.clothSheen,
    required this.pieces,
  });

  final Color background;
  final Color surface;
  final Color surfaceMuted;
  final Color border;
  final Color text;
  final Color textMuted;
  final Color accent;
  final Color accentSoft;
  final Color onAccent;
  final Color danger;

  // Canvas (rendered by Rust).
  final Color board;
  final Color gridMinor;
  final Color gridMajor;
  final Color line;
  final Color skyTop;
  final Color skyBottom;
  final Color ground;
  final Color cloth;
  final Color clothSheen;

  /// Eight piece colours, cycled in project order.
  final List<Color> pieces;

  CanvasColors toCanvas() => CanvasColors(
    boardBackground: board.toARGB32(),
    gridMinor: gridMinor.toARGB32(),
    gridMajor: gridMajor.toARGB32(),
    panelLine: line.toARGB32(),
    selection: accent.toARGB32(),
    skyTop: skyTop.toARGB32(),
    skyBottom: skyBottom.toARGB32(),
    ground: ground.toARGB32(),
    cloth: cloth.toARGB32(),
    clothSheen: clothSheen.toARGB32(),
    pieces: Uint32List.fromList([for (final c in pieces) c.toARGB32()]),
  );

  static const light = Palette(
    background: Color(0xFFE9EBEF),
    surface: Color(0xFFFFFFFF),
    surfaceMuted: Color(0xFFF4F5F8),
    border: Color(0xFFDDE0E6),
    text: Color(0xFF14161B),
    textMuted: Color(0xFF6A7080),
    accent: Color(0xFF6D4AFF),
    accentSoft: Color(0xFFEDE8FF),
    onAccent: Color(0xFFFFFFFF),
    danger: Color(0xFFE5484D),
    board: Color(0xFFF7F8FA),
    gridMinor: Color(0xFFEEF0F4),
    gridMajor: Color(0xFFDFE3EA),
    line: Color(0xFF1B1E26),
    skyTop: Color(0xFFF3F4F7),
    skyBottom: Color(0xFFD5D9E1),
    ground: Color(0xFFE2E5EB),
    cloth: Color(0xFF5E6B8A),
    clothSheen: Color(0xFFE3E8F5),
    pieces: [
      Color(0xFFE4DDFF),
      Color(0xFFD5EEF5),
      Color(0xFFFFE1EC),
      Color(0xFFDDF3E4),
      Color(0xFFFFEBD6),
      Color(0xFFDDE4FF),
      Color(0xFFF2E0FA),
      Color(0xFFE2ECEE),
    ],
  );

  static const dark = Palette(
    background: Color(0xFF08090C),
    surface: Color(0xFF121419),
    surfaceMuted: Color(0xFF1A1D24),
    border: Color(0xFF262A33),
    text: Color(0xFFECEEF3),
    textMuted: Color(0xFF8B92A3),
    accent: Color(0xFF9B87FF),
    accentSoft: Color(0xFF241F47),
    onAccent: Color(0xFF0B0B10),
    danger: Color(0xFFFF6B6B),
    board: Color(0xFF0E1015),
    gridMinor: Color(0xFF151820),
    gridMajor: Color(0xFF1F232D),
    line: Color(0xFFE6E9F0),
    skyTop: Color(0xFF1B1E27),
    skyBottom: Color(0xFF07080B),
    ground: Color(0xFF12141A),
    cloth: Color(0xFF7D89A8),
    clothSheen: Color(0xFFD7DCEF),
    pieces: [
      Color(0xFF2E2655),
      Color(0xFF1C3A43),
      Color(0xFF47233A),
      Color(0xFF1F3D2B),
      Color(0xFF473321),
      Color(0xFF232D52),
      Color(0xFF3B2449),
      Color(0xFF233539),
    ],
  );
}

/// The single UI font (variable, OFL).
const kFont = 'Manrope';

/// Base corner radius; smaller radii derive from it.
const kRadius = 10.0;

abstract final class Space {
  static const xs = 4.0;
  static const s = 8.0;
  static const m = 12.0;
  static const l = 16.0;
  static const xl = 24.0;
}

abstract final class Motion {
  static const fast = Duration(milliseconds: 140);
  static const normal = Duration(milliseconds: 200);
  static const curve = Curves.easeOutCubic;
}

/// Current palette (light or dark), available anywhere in the tree.
class Tokens extends InheritedWidget {
  const Tokens({super.key, required this.dark, required super.child});

  final bool dark;

  static Palette of(BuildContext context) {
    final t = context.dependOnInheritedWidgetOfExactType<Tokens>();
    return (t?.dark ?? false) ? Palette.dark : Palette.light;
  }

  static double radius(BuildContext context, [double factor = 1]) =>
      kRadius * factor;

  @override
  bool updateShouldNotify(Tokens oldWidget) => oldWidget.dark != dark;
}

/// A text style. The font is a variable font, so the weight must also be set
/// as a `wght` variation.
TextStyle ts({
  double? fontSize,
  FontWeight? fontWeight,
  Color? color,
  double? height,
  double? letterSpacing,
  bool tabular = false,
}) {
  return TextStyle(
    fontSize: fontSize,
    fontWeight: fontWeight,
    fontVariations: fontWeight == null
        ? null
        : [FontVariation('wght', fontWeight.value.toDouble())],
    color: color,
    height: height,
    letterSpacing: letterSpacing,
    fontFeatures: tabular ? const [FontFeature.tabularFigures()] : null,
  );
}

ThemeData buildTheme(bool dark) {
  final p = dark ? Palette.dark : Palette.light;
  final brightness = dark ? Brightness.dark : Brightness.light;
  final base = ThemeData(
    brightness: brightness,
    fontFamily: kFont,
    colorScheme: ColorScheme.fromSeed(
      seedColor: p.accent,
      brightness: brightness,
      primary: p.accent,
      onPrimary: p.onAccent,
      surface: p.surface,
      error: p.danger,
    ),
    scaffoldBackgroundColor: p.background,
    dividerColor: p.border,
    splashFactory: InkSparkle.splashFactory,
  );
  final shape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(kRadius * 0.6),
  );
  return base.copyWith(
    textTheme: base.textTheme.apply(bodyColor: p.text, displayColor: p.text),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 400),
      decoration: BoxDecoration(
        color: p.text,
        borderRadius: BorderRadius.circular(kRadius * 0.5),
      ),
      textStyle: ts(color: p.surface, fontSize: 12),
    ),
    sliderTheme: base.sliderTheme.copyWith(
      trackHeight: 4,
      activeTrackColor: p.accent,
      inactiveTrackColor: p.border,
      thumbColor: p.accent,
      overlayColor: p.accent.withValues(alpha: 0.12),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        shape: shape,
        side: BorderSide(color: p.border),
        foregroundColor: p.text,
        padding: const EdgeInsets.symmetric(vertical: 14),
        textStyle: ts(fontSize: 13, fontWeight: FontWeight.w600),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(shape: shape),
    ),
  );
}
