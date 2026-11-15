// Design tokens: the only place where colours, type, radii, spacing and
// motion are defined (docs/ux.md §3). The canvas colours are sent to the
// Rust renderer, so the board and the 3D view match the app.
//
// Three visual directions are defined while the maintainer chooses one;
// the others will be deleted.

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
}

/// A complete visual direction.
class AppStyle {
  const AppStyle({
    required this.name,
    required this.light,
    required this.dark,
    required this.bodyFont,
    required this.displayFont,
    required this.radius,
  });

  final String name;
  final Palette light;
  final Palette dark;
  final String bodyFont;

  /// Used for titles (project name, piece name).
  final String displayFont;

  /// Base corner radius; small and medium radii derive from it.
  final double radius;

  Palette palette(bool isDark) => isDark ? dark : light;

  static const all = [atelier, studio, soft];

  /// Warm and crafted: a tailor's table — cream paper, terracotta, a
  /// serif for titles.
  static const atelier = AppStyle(
    name: 'Atelier',
    bodyFont: 'PlusJakartaSans',
    displayFont: 'Fraunces',
    radius: 16,
    light: Palette(
      background: Color(0xFFEAE3D7),
      surface: Color(0xFFFBF8F2),
      surfaceMuted: Color(0xFFF3EDE3),
      border: Color(0xFFE1D7C6),
      text: Color(0xFF2A2420),
      textMuted: Color(0xFF7A6E62),
      accent: Color(0xFFC2553A),
      accentSoft: Color(0xFFF6E2D9),
      onAccent: Color(0xFFFFFFFF),
      danger: Color(0xFFB83A3A),
      board: Color(0xFFF7F2E9),
      gridMinor: Color(0xFFEFE7DA),
      gridMajor: Color(0xFFE0D4C0),
      line: Color(0xFF3A322C),
      skyTop: Color(0xFFF6F0E6),
      skyBottom: Color(0xFFDDD0BC),
      ground: Color(0xFFE6DCCB),
      cloth: Color(0xFFC77D63),
      clothSheen: Color(0xFFF6E6DA),
      pieces: [
        Color(0xFFF3D8C9),
        Color(0xFFDCE4CF),
        Color(0xFFD6DFEA),
        Color(0xFFEBD5DF),
        Color(0xFFEDE1C3),
        Color(0xFFD2E4E0),
        Color(0xFFE6D6CB),
        Color(0xFFDEDAEC),
      ],
    ),
    dark: Palette(
      background: Color(0xFF14110F),
      surface: Color(0xFF1E1A17),
      surfaceMuted: Color(0xFF28231F),
      border: Color(0xFF3A322C),
      text: Color(0xFFF1E9DE),
      textMuted: Color(0xFFA89A8B),
      accent: Color(0xFFE07A5F),
      accentSoft: Color(0xFF3C2620),
      onAccent: Color(0xFF1E1A17),
      danger: Color(0xFFF07070),
      board: Color(0xFF1A1714),
      gridMinor: Color(0xFF221E1A),
      gridMajor: Color(0xFF2E2823),
      line: Color(0xFFEDE3D6),
      skyTop: Color(0xFF2B2622),
      skyBottom: Color(0xFF141110),
      ground: Color(0xFF1F1B18),
      cloth: Color(0xFFD08A70),
      clothSheen: Color(0xFFF2DACB),
      pieces: [
        Color(0xFF4A3329),
        Color(0xFF34402F),
        Color(0xFF2F3A47),
        Color(0xFF452F3A),
        Color(0xFF463E2A),
        Color(0xFF2B403C),
        Color(0xFF433530),
        Color(0xFF37334A),
      ],
    ),
  );

  /// Professional and precise: cool neutrals, an electric violet, tighter
  /// corners — a creative pro tool.
  static const studio = AppStyle(
    name: 'Studio',
    bodyFont: 'Manrope',
    displayFont: 'Manrope',
    radius: 10,
    light: Palette(
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
    ),
    dark: Palette(
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
    ),
  );

  /// Friendly and calm: soft greens, generous rounding — approachable for
  /// first-time makers.
  static const soft = AppStyle(
    name: 'Soft',
    bodyFont: 'Manrope',
    displayFont: 'Fraunces',
    radius: 22,
    light: Palette(
      background: Color(0xFFE4ECE7),
      surface: Color(0xFFFFFFFF),
      surfaceMuted: Color(0xFFF2F6F3),
      border: Color(0xFFD7E3DB),
      text: Color(0xFF1C2A23),
      textMuted: Color(0xFF64776C),
      accent: Color(0xFF2F8F6B),
      accentSoft: Color(0xFFDCF0E6),
      onAccent: Color(0xFFFFFFFF),
      danger: Color(0xFFD9534F),
      board: Color(0xFFF7FAF8),
      gridMinor: Color(0xFFECF2EE),
      gridMajor: Color(0xFFDCE7E0),
      line: Color(0xFF22332A),
      skyTop: Color(0xFFF2F7F4),
      skyBottom: Color(0xFFD3E2D9),
      ground: Color(0xFFE0EAE4),
      cloth: Color(0xFF7FA88F),
      clothSheen: Color(0xFFEAF5EE),
      pieces: [
        Color(0xFFD7F0E2),
        Color(0xFFE9E2FA),
        Color(0xFFFFF0C9),
        Color(0xFFDDEBFB),
        Color(0xFFFBE0E0),
        Color(0xFFE2F3F1),
        Color(0xFFF3E8DA),
        Color(0xFFE5F0D3),
      ],
    ),
    dark: Palette(
      background: Color(0xFF0D1411),
      surface: Color(0xFF151E19),
      surfaceMuted: Color(0xFF1D2822),
      border: Color(0xFF2A3830),
      text: Color(0xFFE8F1EC),
      textMuted: Color(0xFF93A69B),
      accent: Color(0xFF5CC79B),
      accentSoft: Color(0xFF1B3529),
      onAccent: Color(0xFF0D1411),
      danger: Color(0xFFF07070),
      board: Color(0xFF111A15),
      gridMinor: Color(0xFF17221C),
      gridMajor: Color(0xFF213029),
      line: Color(0xFFE3EEE7),
      skyTop: Color(0xFF1E2A24),
      skyBottom: Color(0xFF0B110E),
      ground: Color(0xFF141D18),
      cloth: Color(0xFF8DB9A0),
      clothSheen: Color(0xFFDDF0E5),
      pieces: [
        Color(0xFF1F3E2F),
        Color(0xFF332C4A),
        Color(0xFF4A3F1E),
        Color(0xFF233649),
        Color(0xFF4A2A2A),
        Color(0xFF1E3D3A),
        Color(0xFF43372A),
        Color(0xFF34401F),
      ],
    ),
  );
}

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

/// Current style and palette, available anywhere in the tree.
class Tokens extends InheritedWidget {
  const Tokens({
    super.key,
    required this.style,
    required this.dark,
    required super.child,
  });

  final AppStyle style;
  final bool dark;

  static Tokens _of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<Tokens>() ??
      const Tokens(style: AppStyle.atelier, dark: false, child: SizedBox());

  static Palette of(BuildContext context) {
    final t = _of(context);
    return t.style.palette(t.dark);
  }

  static AppStyle styleOf(BuildContext context) => _of(context).style;

  static double radius(BuildContext context, [double factor = 1]) =>
      _of(context).style.radius * factor;

  @override
  bool updateShouldNotify(Tokens oldWidget) =>
      oldWidget.style != style || oldWidget.dark != dark;
}

/// A text style. Our fonts are variable fonts, so the weight must also be
/// set as a `wght` variation.
TextStyle ts({
  double? fontSize,
  FontWeight? fontWeight,
  Color? color,
  double? height,
  String? fontFamily,
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
    fontFamily: fontFamily,
    letterSpacing: letterSpacing,
    fontFeatures: tabular ? const [FontFeature.tabularFigures()] : null,
  );
}

ThemeData buildTheme(AppStyle style, bool dark) {
  final p = style.palette(dark);
  final base = ThemeData(
    brightness: dark ? Brightness.dark : Brightness.light,
    fontFamily: style.bodyFont,
    colorScheme: ColorScheme.fromSeed(
      seedColor: p.accent,
      brightness: dark ? Brightness.dark : Brightness.light,
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
    borderRadius: BorderRadius.circular(style.radius * 0.6),
  );
  return base.copyWith(
    textTheme: base.textTheme.apply(bodyColor: p.text, displayColor: p.text),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 400),
      decoration: BoxDecoration(
        color: p.text,
        borderRadius: BorderRadius.circular(style.radius * 0.5),
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
