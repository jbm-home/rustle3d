// Telling the user a new version is out, without getting in the way: at
// most once a day, the app reads https://app.rustle3d.com/latest.json (written
// by the website when it is built for a release). A newer version shows a
// card on the start screen, never while someone works. Nothing is installed
// by the app: the card opens the download page. Offline or on any error the
// app says nothing and asks again at the next launch. The start screen's
// Updates dialog turns the check off or on, and checks at once.

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';

/// Where the website publishes the latest version.
final latestUrl = Uri.parse('https://app.rustle3d.com/latest.json');

/// The download page, in the UI language.
Uri downloadUrl(String language) =>
    Uri.parse('https://app.rustle3d.com/$language/download/');

/// The shortest time between two checks.
const _checkGap = Duration(days: 1);

/// How long a check may take before it is given up.
const _timeout = Duration(seconds: 10);

final _versionPattern = RegExp(r'^\d+(\.\d+)*$');

/// Whether version [a] is newer than [b] (dotted numbers: 26.11.21 is newer
/// than 26.11.20 and 26.12); false if either is not a version.
bool isNewer(String a, String b) {
  if (!_versionPattern.hasMatch(a) || !_versionPattern.hasMatch(b)) {
    return false;
  }
  final x = a.split('.').map(int.parse).toList();
  final y = b.split('.').map(int.parse).toList();
  for (var i = 0; i < x.length || i < y.length; i++) {
    final p = i < x.length ? x[i] : 0;
    final q = i < y.length ? y[i] : 0;
    if (p != q) return p > q;
  }
  return false;
}

/// Ask [uri] for the latest version; null offline or on any error.
Future<String?> fetchLatest(Uri uri) async {
  final client = HttpClient()
    ..connectionTimeout = _timeout
    ..userAgent = 'Rustle3D';
  try {
    final request = await client.getUrl(uri).timeout(_timeout);
    final response = await request.close().timeout(_timeout);
    if (response.statusCode != HttpStatus.ok) return null;
    final body = await response
        .transform(utf8.decoder)
        .join()
        .timeout(_timeout);
    final json = jsonDecode(body);
    return json is Map && json['version'] is String
        ? json['version'] as String
        : null;
  } on IOException {
    return null;
  } on TimeoutException {
    return null;
  } on FormatException {
    return null;
  } finally {
    client.close(force: true);
  }
}

/// The update check's state, remembered in the user's library folder
/// (`update.json`): when the site was last asked, the latest version it
/// gave, the version the user chose to skip, and whether the check is off.
class UpdateCheck extends ChangeNotifier {
  UpdateCheck(
    this.libraryDir, {
    required this.current,
    this.fetch = fetchLatest,
  }) : _state = _read(File('$libraryDir/update.json'));

  final String libraryDir;

  /// This app's version.
  final String current;

  /// How the latest version is asked (replaced in tests).
  final Future<String?> Function(Uri) fetch;

  Map<String, Object?> _state;

  File get _file => File('$libraryDir/update.json');

  /// Whether the app checks for updates.
  bool get enabled => _state['off'] != true;

  set enabled(bool on) {
    _save({..._state, 'off': !on});
    if (on) unawaited(check(force: true));
  }

  /// The newer version to offer, null if there is none, the user skipped
  /// it, or the check is off.
  String? get available {
    final latest = _state['latest'];
    return enabled &&
            latest is String &&
            latest != _state['skipped'] &&
            isNewer(latest, current)
        ? latest
        : null;
  }

  /// Ask the site, unless it was asked less than a day ago (or [force]).
  /// The request says which version and system ask, nothing else.
  Future<void> check({bool force = false, DateTime? now}) async {
    if (!enabled) return;
    final today = now ?? DateTime.now();
    final last = DateTime.tryParse(_state['checked'] as String? ?? '');
    if (!force &&
        last != null &&
        !today.isBefore(last) &&
        today.difference(last) < _checkGap) {
      return;
    }
    await _ask(today);
  }

  /// Ask the site now, even with the check off (the user asked): the
  /// latest version, null if the site could not be reached.
  Future<String?> checkNow({DateTime? now}) => _ask(now ?? DateTime.now());

  Future<String?> _ask(DateTime today) async {
    final latest = await fetch(
      latestUrl.replace(
        queryParameters: {'v': current, 'os': Platform.operatingSystem},
      ),
    );
    if (latest == null || !_versionPattern.hasMatch(latest)) return null;
    _save({..._state, 'checked': today.toIso8601String(), 'latest': latest});
    return latest;
  }

  /// Don't offer this version again (a later one is offered).
  void skip() {
    if (available case final version?) _save({..._state, 'skipped': version});
  }

  static Map<String, Object?> _read(File file) {
    try {
      final json = jsonDecode(file.readAsStringSync());
      return json is Map<String, Object?> ? json : {};
    } on FileSystemException {
      return {};
    } on FormatException {
      return {};
    }
  }

  void _save(Map<String, Object?> state) {
    _state = state;
    try {
      _file
        ..createSync(recursive: true)
        ..writeAsStringSync(jsonEncode(state));
    } on FileSystemException {
      // Not remembered: at worst the site is asked again next launch.
    }
    notifyListeners();
  }
}

/// The start screen's card for a new version: a button to the download
/// page, and a way to skip this version.
class UpdateCard extends StatelessWidget {
  const UpdateCard({
    super.key,
    required this.version,
    required this.onDownload,
    required this.onSkip,
  });

  final String version;
  final VoidCallback onDownload;
  final VoidCallback onSkip;

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
          Icon(Icons.system_update_alt_rounded, color: p.accent, size: 22),
          const SizedBox(width: Space.m),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l.updateTitle(version),
                  style: ts(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: p.text,
                  ),
                ),
                const SizedBox(height: 2),
                Text(l.updateText, style: ts(fontSize: 13, color: p.textMuted)),
              ],
            ),
          ),
          const SizedBox(width: Space.m),
          TextButton(onPressed: onSkip, child: Text(l.updateSkip)),
          const SizedBox(width: Space.s),
          FilledButton.icon(
            onPressed: onDownload,
            icon: const Icon(Icons.open_in_new_rounded, size: 16),
            label: Text(l.updateDownload),
          ),
        ],
      ),
    );
  }
}

/// The Updates dialog of the start screen: the daily check on or off, and
/// a check at once, with what it found.
Future<void> showUpdates(
  BuildContext context, {
  required UpdateCheck updates,
  required VoidCallback onDownload,
}) {
  return showDialog<void>(
    context: context,
    builder: (_) => _Updates(updates: updates, onDownload: onDownload),
  );
}

/// Where a check started from the dialog stands.
enum _Checked { no, running, failed, found }

class _Updates extends StatefulWidget {
  const _Updates({required this.updates, required this.onDownload});

  final UpdateCheck updates;
  final VoidCallback onDownload;

  @override
  State<_Updates> createState() => _UpdatesState();
}

class _UpdatesState extends State<_Updates> {
  _Checked _checked = _Checked.no;
  String? _latest;

  void _changed() => setState(() {});

  @override
  void initState() {
    super.initState();
    widget.updates.addListener(_changed);
  }

  @override
  void dispose() {
    widget.updates.removeListener(_changed);
    super.dispose();
  }

  Future<void> _check() async {
    setState(() => _checked = _Checked.running);
    final latest = await widget.updates.checkNow();
    if (!mounted) return;
    setState(() {
      _latest = latest;
      _checked = latest == null ? _Checked.failed : _Checked.found;
    });
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final updates = widget.updates;
    final newer = switch (_latest) {
      final latest? when isNewer(latest, updates.current) => latest,
      _ => null,
    };
    Widget note(String text, {bool strong = false}) => Text(
      text,
      key: const ValueKey('updatesResult'),
      style: ts(
        fontSize: 13,
        height: 1.45,
        fontWeight: strong ? FontWeight.w600 : FontWeight.w400,
        color: strong ? p.text : p.textMuted,
      ),
    );
    return AlertDialog(
      title: Text(l.updatesTitle),
      content: SizedBox(
        width: 440,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              l.appVersion(updates.current),
              style: ts(fontSize: 13, color: p.textMuted),
            ),
            const SizedBox(height: Space.s),
            SwitchListTile(
              key: const ValueKey('updatesAuto'),
              contentPadding: EdgeInsets.zero,
              dense: true,
              title: Text(l.updateAuto, style: ts(fontSize: 13.5)),
              subtitle: Text(
                l.updateAutoHint,
                style: ts(fontSize: 12.5, height: 1.4, color: p.textMuted),
              ),
              value: updates.enabled,
              onChanged: (on) => updates.enabled = on,
            ),
            const SizedBox(height: Space.m),
            Row(
              children: [
                OutlinedButton.icon(
                  key: const ValueKey('updatesCheckNow'),
                  onPressed: _checked == _Checked.running ? null : _check,
                  icon: const Icon(Icons.refresh_rounded, size: 18),
                  label: Text(l.updateCheckNow),
                ),
                const SizedBox(width: Space.m),
                if (_checked == _Checked.running)
                  const SizedBox.square(
                    dimension: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
              ],
            ),
            if (_checked == _Checked.failed) ...[
              const SizedBox(height: Space.s),
              note(l.updateFailed),
            ],
            if (_checked == _Checked.found) ...[
              const SizedBox(height: Space.s),
              if (newer != null) ...[
                note(l.updateTitle(newer), strong: true),
                const SizedBox(height: Space.s),
                Align(
                  alignment: Alignment.centerLeft,
                  child: FilledButton.icon(
                    onPressed: widget.onDownload,
                    icon: const Icon(Icons.open_in_new_rounded, size: 16),
                    label: Text(l.updateDownload),
                  ),
                ),
              ] else
                note(l.updateLatest),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l.close),
        ),
      ],
    );
  }
}
