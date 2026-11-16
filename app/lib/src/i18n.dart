// The UI language (docs/architecture.md §7.7): English by default, chosen
// by the user and remembered in their library folder. Messages come from
// the ARB catalog in lib/l10n; errors from the core are translated from
// their stable code and arguments, with the English text as fallback.

import 'dart:io';

import 'package:intl/intl.dart';
import 'package:material_ui/material_ui.dart';

import '../l10n/app_localizations.dart';
import 'design/tokens.dart';
import 'rust/api/engine.dart';

export '../l10n/app_localizations.dart' show L;

/// The languages offered, each named in itself.
const languages = [('en', 'English'), ('fr', 'Français')];

/// The chosen language, remembered in the user's library folder.
class LanguageMemory {
  const LanguageMemory(this.libraryDir);

  final String libraryDir;

  File get _file => File('$libraryDir/language');

  /// The saved language code, English if none (or unreadable).
  String get code {
    try {
      final saved = _file.readAsStringSync().trim();
      return languages.any((l) => l.$1 == saved) ? saved : 'en';
    } on FileSystemException {
      return 'en';
    }
  }

  set code(String code) {
    try {
      _file
        ..createSync(recursive: true)
        ..writeAsStringSync(code);
    } on FileSystemException {
      // Not remembered: the app starts in English next time, nothing worse.
    }
  }
}

extension Messages on BuildContext {
  L get l => L.of(this);

  /// [value] with [digits] decimals, in the UI language's notation.
  String number(num value, [int digits = 0]) =>
      NumberFormat.decimalPatternDigits(
        locale: Localizations.localeOf(this).toString(),
        decimalDigits: digits,
      ).format(value);

  /// An error from the core, in the UI language.
  String error(ApiError e) {
    final l = this.l;
    String arg(String name) => e.args[name] ?? '';
    return switch (e.code) {
      'not_found' => l.errNotFound(arg('id')),
      'linked_mirror' => l.errLinkedMirror(arg('panel')),
      'sewn' => l.errSewn,
      'nothing_to_undo' => l.errNothingToUndo,
      'nothing_to_redo' => l.errNothingToRedo,
      'nothing_to_simulate' => l.errNothingToSimulate,
      'invalid.inverted' => l.errInverted,
      'invalid.range' => l.errRange(arg('min'), arg('max')),
      'invalid.not_positive' => l.errNotPositive,
      'invalid.negative' => l.errNegative,
      'invalid.empty' => l.errEmpty,
      'invalid.not_simple' => l.errNotSimple,
      'file.io' || 'write_failed' => l.errFileIo,
      'file.not_a_project' => l.errNotAProject,
      'file.too_new' || 'file.unsupported_version' => l.errTooNew,
      'file.corrupted' || 'file.json' => l.errCorrupted,
      'file.already_exists' => l.errAlreadyExists,
      _ => e.text,
    };
  }

  /// A body preset's name for display: the built-in sizes are translated,
  /// the user's own presets are shown as they named them.
  String presetName(Map<String, dynamic> preset) {
    final name = preset['name'] as String;
    if (preset['builtin'] != true) return name;
    return switch (name.split(' ')) {
      ['Women', final size] => l.presetWomen(size),
      ['Men', final size] => l.presetMen(size),
      _ => name,
    };
  }
}

/// The piece names templates give, translated when the project is made
/// (they are the user's data afterwards).
Map<String, String> pieceNames(L l) => {
  'Front': l.pieceFront,
  'Back': l.pieceBack,
  'Back left': l.pieceBackLeft,
  'Back right': l.pieceBackRight,
  'Sleeve left': l.pieceSleeveLeft,
  'Sleeve right': l.pieceSleeveRight,
};

/// Pick the UI language.
class LanguageButton extends StatelessWidget {
  const LanguageButton({
    super.key,
    required this.current,
    required this.onChanged,
  });

  final String current;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return PopupMenuButton<String>(
      tooltip: context.l.languageTooltip,
      initialValue: current,
      onSelected: onChanged,
      itemBuilder: (_) => [
        for (final (code, name) in languages)
          PopupMenuItem(value: code, child: Text(name)),
      ],
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.translate_rounded, size: 18, color: p.textMuted),
            const SizedBox(width: 4),
            Text(
              current.toUpperCase(),
              style: ts(
                fontSize: 12,
                fontWeight: FontWeight.w600,
                color: p.textMuted,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
