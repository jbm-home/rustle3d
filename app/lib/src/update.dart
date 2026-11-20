// Telling the user a new version is out, without getting in the way: at
// most once a day, the app reads https://rustle3d.com/latest.json (written
// by the website when it is built for a release). A newer version shows a
// card on the start screen, never while someone works. Nothing is installed
// by the app: the card opens the download page. Offline or on any error the
// app says nothing and asks again at the next launch. The user can turn the
// check off on the start screen.

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';

/// Where the website publishes the latest version.
final latestUrl = Uri.parse('https://rustle3d.com/latest.json');

/// The download page, in the UI language.
Uri downloadUrl(String language) =>
    Uri.parse('https://rustle3d.com/$language/download/');

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
    final latest = await fetch(
      latestUrl.replace(
        queryParameters: {'v': current, 'os': Platform.operatingSystem},
      ),
    );
    if (latest == null || !_versionPattern.hasMatch(latest)) return;
    _save({..._state, 'checked': today.toIso8601String(), 'latest': latest});
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
