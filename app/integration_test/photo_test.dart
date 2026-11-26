// Photos in the app: the Photo button opens the photo scene, whose panel
// sets the light, the lens and the size while the 3D view shows the photo
// as it will be; the photo then refines with its progress, it can be
// stopped, saved as PNG and put away, and the scene is saved with the
// project.

import 'dart:io';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('render a photo, stop it, save it and close it', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    final engine = Engine.open(
      path: await store.create(
        name: 'Skirt',
        body: store.standardBody(),
        template: 'a_line_skirt',
      ),
    );
    await engine.resetSimulation();
    File('$dir/tour-seen').createSync();
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    for (var i = 0; i < 20; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }

    // The views redraw every frame: pump a while instead of settling.
    Future<void> frames([int n = 10]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 50));
      }
    }

    await tester.tap(find.byKey(const ValueKey('photoButton')));
    await frames();
    expect(find.text('Photo scene'), findsOneWidget);
    // The view already shows the photo, refining live.
    expect(engine.photoStatus(), isNotNull);
    expect(find.byKey(const ValueKey('photoProgress')), findsNothing);
    // The test window is small: bring each control into view first.
    Future<void> tap(Finder f) async {
      await tester.ensureVisible(f);
      await tester.pump();
      await tester.tap(f);
    }

    await tap(find.text('Quick'));
    // A place offers the environments, then back to a studio.
    await tap(find.text('Place'));
    await frames(2);
    expect(find.byKey(const ValueKey('photoEnvironment')), findsOneWidget);
    expect(find.text('Ferndale'), findsOneWidget);
    await tap(find.text('Soft'));
    await frames(2);
    expect(find.byKey(const ValueKey('photoEnvironment')), findsNothing);
    expect(find.text('A first look in seconds'), findsOneWidget);
    // A longer lens.
    final lens = find.descendant(
      of: find.byKey(const ValueKey('photoFocal')),
      matching: find.byType(Slider),
    );
    await tester.ensureVisible(lens);
    await tester.pump();
    await tester.drag(lens, const Offset(60, 0));
    await frames(2);
    // Default settings puts the lens back.
    await tap(find.byKey(const ValueKey('photoDefaults')));
    await frames(2);
    expect(find.text('38 mm'), findsOneWidget);
    await tester.ensureVisible(lens);
    await tester.pump();
    await tester.drag(lens, const Offset(60, 0));
    await frames(2);
    final focal = engine.photoScene();
    expect(focal.light, 'studio', reason: 'not saved before closing');
    await tap(find.byKey(const ValueKey('photoRender')));
    await frames();
    // Rendering saved the scene in the project.
    final saved = engine.photoScene();
    expect(saved.light, 'soft');
    expect(saved.focalMm, greaterThan(40));

    // The 3D view shows the photo with its progress, and no live controls.
    expect(engine.photoStatus(), isNotNull);
    expect(find.byKey(const ValueKey('photoProgress')), findsOneWidget);
    expect(find.byKey(const ValueKey('photoButton')), findsNothing);
    final status = engine.photoStatus()!;
    expect(status.total, 16);
    expect(status.width, greaterThan(0));

    // A few samples, then stop: the image keeps them.
    while (engine.photoStatus()!.samples < 2) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    engine.stopPhoto();
    while (!engine.photoStatus()!.finished) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    await tester.pump(const Duration(milliseconds: 50));
    final stopped = engine.photoStatus()!;
    expect(stopped.error, isNull);
    expect(stopped.samples, lessThanOrEqualTo(16));
    expect(find.textContaining(RegExp('Stopped|Done')), findsOneWidget);

    await engine.savePhoto(path: '$dir/photo.png');
    final png = File('$dir/photo.png').readAsBytesSync();
    expect(png.sublist(1, 4), 'PNG'.codeUnits);

    // A fixed size, with the advanced settings.
    final fixed = engine.startPhoto(
      request: const PhotoRequest(
        quality: 'quick',
        lighting: 'dramatic',
        width: 64,
        height: 48,
        samples: 2,
        bounces: 1,
        denoise: false,
        turnDeg: 0,
        brightness: 1,
        placeSizeM: 6,
      ),
    );
    expect((fixed.width, fixed.height, fixed.total), (64, 48, 2));
    while (!engine.photoStatus()!.finished) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    expect(engine.photoStatus()!.samples, 2);
    expect(
      () => engine.startPhoto(
        request: const PhotoRequest(
          quality: 'quick',
          lighting: 'studio',
          width: 4096,
          height: 4096,
          denoise: true,
          turnDeg: 0,
          brightness: 1,
          placeSizeM: 6,
        ),
      ),
      throwsA(isA<ApiError>().having((e) => e.code, 'code', 'photo_too_large')),
    );

    // Lit by a place: the built-in environments, and an unknown one
    // refused.
    expect(engine.environments(), containsAll(['ferndale', 'neon']));
    final lit = engine.startPhoto(
      request: const PhotoRequest(
        quality: 'quick',
        lighting: 'studio',
        width: 48,
        height: 48,
        samples: 1,
        denoise: true,
        environment: 'neon',
        turnDeg: 45,
        brightness: 1.5,
        placeSizeM: 10,
      ),
    );
    expect(lit.total, 1);
    while (!engine.photoStatus()!.finished) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    expect(engine.photoStatus()!.error, isNull);
    expect(
      () => engine.startPhoto(
        request: const PhotoRequest(
          quality: 'quick',
          lighting: 'studio',
          width: 48,
          height: 48,
          denoise: true,
          environment: 'sunset',
          turnDeg: 0,
          brightness: 1,
          placeSizeM: 6,
        ),
      ),
      throwsA(isA<ApiError>()),
    );

    // Putting the photo away goes back to the scene's live photo.
    await tester.tap(find.byKey(const ValueKey('photoClose')));
    await tester.pump();
    expect(find.byKey(const ValueKey('photoProgress')), findsNothing);
    expect(engine.photoStatus(), isNotNull);
    await tester.tap(find.byKey(const ValueKey('sceneClose')));
    await tester.pump();
    expect(engine.photoStatus(), isNull);
    expect(find.byKey(const ValueKey('photoButton')), findsOneWidget);
    expect(engine.photoScene().light, 'soft');
    await expectLater(
      engine.savePhoto(path: '$dir/none.png'),
      throwsA(isA<ApiError>()),
    );
  });
}
