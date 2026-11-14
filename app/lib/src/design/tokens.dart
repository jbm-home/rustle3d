// Design tokens: the only place where colours, radii, spacing and motion
// are defined (docs/ux.md §3). Canvas colours match r3d_render::Theme.

import 'package:material_ui/material_ui.dart';

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
    required this.danger,
  });

  final Color background;
  final Color surface;
  final Color surfaceMuted;
  final Color border;
  final Color text;
  final Color textMuted;
  final Color accent;
  final Color accentSoft;
  final Color danger;

  static const light = Palette(
    background: Color(0xFFEFEDE8),
    surface: Color(0xFFFFFFFF),
    surfaceMuted: Color(0xFFF6F5F2),
    border: Color(0xFFE2DFD8),
    text: Color(0xFF1F1E1C),
    textMuted: Color(0xFF6E6B64),
    accent: Color(0xFF5B5BD6),
    accentSoft: Color(0xFFEDEDFC),
    danger: Color(0xFFD64545),
  );

  static const dark = Palette(
    background: Color(0xFF0E0E10),
    surface: Color(0xFF18181B),
    surfaceMuted: Color(0xFF202024),
    border: Color(0xFF2B2B30),
    text: Color(0xFFEDEBE7),
    textMuted: Color(0xFF9B988F),
    accent: Color(0xFF8B8BF5),
    accentSoft: Color(0xFF26264A),
    danger: Color(0xFFF07070),
  );
}

abstract final class Radii {
  static const small = 8.0;
  static const medium = 12.0;
  static const large = 16.0;
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

/// Palette lookup from anywhere in the tree.
class Tokens extends InheritedWidget {
  const Tokens({super.key, required this.palette, required super.child});

  final Palette palette;

  static Palette of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<Tokens>()?.palette ??
      Palette.light;

  @override
  bool updateShouldNotify(Tokens oldWidget) => oldWidget.palette != palette;
}

ThemeData buildTheme(Palette p, Brightness brightness) {
  final base = ThemeData(
    brightness: brightness,
    colorScheme: ColorScheme.fromSeed(
      seedColor: p.accent,
      brightness: brightness,
      primary: p.accent,
      surface: p.surface,
      error: p.danger,
    ),
    scaffoldBackgroundColor: p.background,
    dividerColor: p.border,
    splashFactory: InkSparkle.splashFactory,
  );
  return base.copyWith(
    textTheme: base.textTheme.apply(bodyColor: p.text, displayColor: p.text),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 400),
      decoration: BoxDecoration(
        color: p.text,
        borderRadius: BorderRadius.circular(Radii.small),
      ),
      textStyle: TextStyle(color: p.surface, fontSize: 12),
    ),
    sliderTheme: base.sliderTheme.copyWith(
      trackHeight: 4,
      activeTrackColor: p.accent,
      inactiveTrackColor: p.border,
      thumbColor: p.accent,
      overlayColor: p.accent.withValues(alpha: 0.12),
    ),
  );
}
