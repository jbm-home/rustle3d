// Asking for donations without getting in the way (docs/ux.md §1): a
// permanent "Support Rustle3D" link on the start screen, and a card that
// comes back at most once a quarter, only on the start screen (never while
// someone works), only after the app has been used for a while, and never
// again once the user says so.

import 'dart:convert';
import 'dart:io';

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';

/// The donation page. One address the site controls, so the platform
/// behind it can change without a new release of the app.
final donateUrl = Uri.parse('https://rustle3d.com/donate');

/// Launches before the card may show for the first time.
const _launchesBeforeFirstReminder = 10;

/// The shortest time between two reminders.
const _reminderGap = Duration(days: 90);

/// The donation reminder's state, remembered in the user's library folder
/// (`support.json`): how many times the app was launched, when the card may
/// show again, and whether the user turned it off.
class SupportMemory {
  SupportMemory(this.libraryDir);

  final String libraryDir;

  File get _file => File('$libraryDir/support.json');

  /// Whether the card shows during this launch, decided once by [launched].
  bool remindNow = false;

  /// Count one launch and decide whether this one shows the card. A launch
  /// that shows it already pushes the next reminder a quarter away, so
  /// ignoring the card is as good as "Not now".
  void launched({required bool hasProjects, DateTime? now}) {
    final today = now ?? DateTime.now();
    final state = _read();
    final launches = (state['launches'] as int? ?? 0) + 1;
    final next = DateTime.tryParse(state['next'] as String? ?? '');
    final off = state['off'] == true;
    remindNow =
        !off &&
        hasProjects &&
        launches >= _launchesBeforeFirstReminder &&
        (next == null || !today.isBefore(next));
    final nextReminder = remindNow ? today.add(_reminderGap) : next;
    _write({
      'launches': launches,
      'next': nextReminder?.toIso8601String(),
      'off': off,
    });
  }

  /// The card is closed for this launch; it comes back in a quarter.
  void later() => remindNow = false;

  /// The card never shows again (the link stays).
  void turnOff() {
    remindNow = false;
    _write({..._read(), 'off': true});
  }

  Map<String, Object?> _read() {
    try {
      final json = jsonDecode(_file.readAsStringSync());
      return json is Map<String, Object?> ? json : {};
    } on FileSystemException {
      return {};
    } on FormatException {
      return {};
    }
  }

  void _write(Map<String, Object?> state) {
    try {
      _file
        ..createSync(recursive: true)
        ..writeAsStringSync(jsonEncode(state));
    } on FileSystemException {
      // Not remembered: at worst the card shows again, never more than once
      // per launch.
    }
  }
}

/// The start screen's donation card: what donations pay for, a button to
/// the donation page, and two ways to close it.
class SupportCard extends StatelessWidget {
  const SupportCard({
    super.key,
    required this.onDonate,
    required this.onLater,
    required this.onTurnOff,
  });

  final VoidCallback onDonate;
  final VoidCallback onLater;
  final VoidCallback onTurnOff;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return Container(
      margin: const EdgeInsets.only(top: Space.l),
      padding: const EdgeInsets.fromLTRB(Space.l, Space.m, Space.s, Space.m),
      decoration: BoxDecoration(
        color: p.accentSoft,
        borderRadius: BorderRadius.circular(kRadius),
      ),
      child: Row(
        children: [
          Icon(Icons.favorite_border_rounded, color: p.accent, size: 22),
          const SizedBox(width: Space.m),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l.supportTitle,
                  style: ts(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: p.text,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  l.supportText,
                  style: ts(fontSize: 13, color: p.textMuted),
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.m),
          TextButton(onPressed: onTurnOff, child: Text(l.supportTurnOff)),
          TextButton(onPressed: onLater, child: Text(l.supportLater)),
          const SizedBox(width: Space.s),
          FilledButton.icon(
            onPressed: onDonate,
            icon: const Icon(Icons.open_in_new_rounded, size: 16),
            label: Text(l.supportDonate),
          ),
        ],
      ),
    );
  }
}
