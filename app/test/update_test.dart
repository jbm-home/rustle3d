// The update check: when the site is asked and which version is offered.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:rustle3d/src/update.dart';

void main() {
  late Directory dir;
  setUp(() => dir = Directory.systemTemp.createTempSync('r3d-update'));
  tearDown(() => dir.deleteSync(recursive: true));

  final day = DateTime(2027, 1, 1);

  /// A check of version 26.11.20 against a site that answers [latest],
  /// counting the requests in [asked].
  UpdateCheck checker(String? latest, List<Uri> asked) => UpdateCheck(
    dir.path,
    current: '26.11.20',
    fetch: (uri) async {
      asked.add(uri);
      return latest;
    },
  );

  test('versions compare number by number', () {
    expect(isNewer('26.11.21', '26.11.20'), isTrue);
    expect(isNewer('26.12', '26.11.20'), isTrue);
    expect(isNewer('26.11.20', '26.11.20'), isFalse);
    expect(isNewer('26.11.20.0', '26.11.20'), isFalse);
    expect(isNewer('26.9.30', '26.11.20'), isFalse);
    expect(isNewer('27.1.2-beta', '26.11.20'), isFalse);
  });

  test('a newer version is offered, the same one is not', () async {
    final asked = <Uri>[];
    final newer = checker('26.11.21', asked);
    await newer.check(now: day);
    expect(newer.available, '26.11.21');
    expect(asked.single.queryParameters['v'], '26.11.20');
    expect(asked.single.queryParameters['os'], Platform.operatingSystem);

    final same = checker('26.11.20', asked);
    await same.check(force: true, now: day);
    expect(same.available, isNull);
  });

  test('the site is asked at most once a day', () async {
    final asked = <Uri>[];
    await checker('26.11.21', asked).check(now: day);
    await checker(
      '26.11.21',
      asked,
    ).check(now: day.add(const Duration(hours: 23)));
    expect(asked, hasLength(1));
    await checker(
      '26.11.21',
      asked,
    ).check(now: day.add(const Duration(days: 1)));
    expect(asked, hasLength(2));
  });

  test('offline: nothing offered, asked again next launch', () async {
    final asked = <Uri>[];
    final offline = checker(null, asked);
    await offline.check(now: day);
    expect(offline.available, isNull);
    await checker('26.11.21', asked).check(now: day);
    expect(asked, hasLength(2));
  });

  test('a skipped version is not offered again, a later one is', () async {
    final asked = <Uri>[];
    final first = checker('26.11.21', asked);
    await first.check(now: day);
    first.skip();
    expect(first.available, isNull);
    expect(checker('26.11.21', asked).available, isNull);
    final later = checker('26.12.1', asked);
    await later.check(force: true, now: day);
    expect(later.available, '26.12.1');
  });

  test('turned off: no request, nothing offered', () async {
    final asked = <Uri>[];
    final first = checker('26.11.21', asked);
    await first.check(now: day);
    first.enabled = false;
    expect(first.available, isNull);
    final next = checker('26.11.21', asked);
    await next.check(now: day.add(const Duration(days: 5)));
    expect(asked, hasLength(1));
    expect(next.enabled, isFalse);
  });

  test('an unreadable file starts over', () async {
    File('${dir.path}/update.json').writeAsStringSync('not json');
    final asked = <Uri>[];
    final check = checker('26.11.21', asked);
    await check.check(now: day);
    expect(check.available, '26.11.21');
  });
}
