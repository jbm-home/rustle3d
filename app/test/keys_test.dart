// Shortcuts are written with the macOS symbols and shown as the platform's
// keys, named in the UI language.

import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:rustle3d/src/i18n.dart';

Future<String> shown(WidgetTester tester, String locale, String text) async {
  late String out;
  await tester.pumpWidget(
    MaterialApp(
      locale: Locale(locale),
      localizationsDelegates: const [
        L.delegate,
        ...GlobalMaterialLocalizations.delegates,
      ],
      supportedLocales: L.supportedLocales,
      home: Builder(
        builder: (context) {
          out = context.keys(text);
          return const SizedBox();
        },
      ),
    ),
  );
  return out;
}

void main() {
  testWidgets('macOS keeps the symbols', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    expect(await shown(tester, 'en', 'Redo  ⇧⌘Z'), 'Redo  ⇧⌘Z');
    debugDefaultTargetPlatformOverride = null;
  });

  testWidgets('elsewhere, key names in the UI language', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    expect(await shown(tester, 'en', 'Redo  ⇧⌘Z'), 'Redo  Ctrl+Shift+Z');
    expect(
      await shown(tester, 'en', '⌥-click to pin · ⌫ deletes'),
      'Alt-click to pin · Delete deletes',
    );
    expect(await shown(tester, 'fr', 'Maintenez **⇧**'), 'Maintenez **Maj**');
    expect(await shown(tester, 'fr', '⌘-glisser'), 'Ctrl-glisser');
    debugDefaultTargetPlatformOverride = null;
  });
}
