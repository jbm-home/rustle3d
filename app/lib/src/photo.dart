// Photos: the photo scene's panel beside the 3D view, which shows the
// photo as it will be while it is set, and the bar over the 3D view while
// the photo refines (docs/specs/14-photoreal-rendering.md,
// docs/specs/35-environment-lighting.md).

import 'dart:math' as math;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'rust/api/engine.dart';

/// Image sizes offered; (0, 0) is the 3D view's own.
const photoSizes = [
  (0, 0),
  (1920, 1080),
  (1080, 1350),
  (2048, 2048),
  (3840, 2160),
];

/// The focal lengths offered (mm on a 24 × 36 frame).
const _focalMin = 18.0;
const _focalMax = 200.0;

/// A place's radius around the model (m).
const _placeMin = 3.0;
const _placeMax = 100.0;

/// How long to refine a photo, as last chosen (the scene itself is the
/// project's).
class PhotoChoice {
  const PhotoChoice({
    this.quality = 'good',
    this.samples,
    this.bounces,
    this.denoise = true,
  });

  /// `quick`, `good` or `best`.
  final String quality;

  /// Instead of the quality's (advanced).
  final int? samples;
  final int? bounces;
  final bool denoise;

  /// The photo of `scene` refined this way.
  PhotoRequest request(SceneSettings scene) {
    final lit = scene.light == 'environment';
    return PhotoRequest(
      quality: quality,
      lighting: lit ? 'studio' : scene.light,
      width: scene.width,
      height: scene.height,
      samples: samples,
      bounces: bounces,
      denoise: denoise,
      environment: lit ? scene.environment : null,
      turnDeg: scene.turnDeg,
      brightness: scene.brightness,
      placeSizeM: scene.placeSizeM,
    );
  }
}

SceneSettings _with(
  SceneSettings s, {
  String? light,
  String? environment,
  double? turnDeg,
  double? brightness,
  double? placeSizeM,
  double? focalMm,
  (int, int)? size,
}) => SceneSettings(
  light: light ?? s.light,
  environment: environment ?? s.environment,
  turnDeg: turnDeg ?? s.turnDeg,
  brightness: brightness ?? s.brightness,
  placeSizeM: placeSizeM ?? s.placeSizeM,
  focalMm: focalMm ?? s.focalMm,
  width: size?.$1 ?? s.width,
  height: size?.$2 ?? s.height,
);

/// The photo scene's settings beside the 3D view: every change shows in
/// the view at once; [onShoot] renders the photo with [choice].
class ScenePanel extends StatefulWidget {
  const ScenePanel({
    super.key,
    required this.engine,
    required this.scene,
    required this.choice,
    required this.onScene,
    required this.onChoice,
    required this.onShoot,
    required this.onClose,
  });

  final Engine engine;
  final SceneSettings scene;
  final PhotoChoice choice;
  final ValueChanged<SceneSettings> onScene;
  final ValueChanged<PhotoChoice> onChoice;
  final VoidCallback onShoot;
  final VoidCallback onClose;

  @override
  State<ScenePanel> createState() => _ScenePanelState();
}

class _ScenePanelState extends State<ScenePanel> {
  late List<String> _environments = _listEnvironments();
  late final _samples = TextEditingController(
    text: widget.choice.samples?.toString() ?? '',
  );
  late final _bounces = TextEditingController(
    text: widget.choice.bounces?.toString() ?? '',
  );

  @override
  void dispose() {
    _samples.dispose();
    _bounces.dispose();
    super.dispose();
  }

  List<String> _listEnvironments() {
    try {
      return widget.engine.environments();
    } on ApiError {
      return const [];
    }
  }

  /// A built-in environment's name, capitalised, or a file's name.
  String _environmentName(String id) {
    if (id.contains('/')) return id.split('/').last;
    return id.isEmpty ? id : id[0].toUpperCase() + id.substring(1);
  }

  void _scene(SceneSettings s) => widget.onScene(s);

  void _choice({String? quality, bool? denoise}) => widget.onChoice(
    PhotoChoice(
      quality: quality ?? widget.choice.quality,
      samples: int.tryParse(_samples.text),
      bounces: int.tryParse(_bounces.text),
      denoise: denoise ?? widget.choice.denoise,
    ),
  );

  void _light(String light) {
    final s = widget.scene;
    final environment = light == 'environment' && s.environment.isEmpty
        ? (_environments.isEmpty ? '' : _environments.first)
        : s.environment;
    _scene(_with(s, light: light, environment: environment));
  }

  Future<void> _addEnvironment() async {
    final l = context.l;
    final messenger = ScaffoldMessenger.of(context);
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(label: l.photoEnvironmentFiles, extensions: const ['hdr']),
      ],
    );
    if (file == null || !mounted) return;
    try {
      final path = await widget.engine.adoptEnvironment(path: file.path);
      if (!mounted) return;
      setState(() => _environments = _listEnvironments());
      _scene(_with(widget.scene, light: 'environment', environment: path));
    } on ApiError catch (e) {
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(
          content: Text(context.error(e)),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    final p = Tokens.of(context);
    final s = widget.scene;
    final c = widget.choice;
    final qualities = [
      ('quick', l.photoQuick, l.photoQuickHint, '16'),
      ('good', l.photoGood, l.photoGoodHint, '128'),
      ('best', l.photoBest, l.photoBestHint, '512'),
    ];
    final lightings = [
      ('studio', l.lightStudio, l.lightStudioHint),
      ('soft', l.lightSoft, l.lightSoftHint),
      ('dramatic', l.lightDramatic, l.lightDramaticHint),
      ('environment', l.lightEnvironment, l.lightEnvironmentHint),
    ];
    Widget label(String text) => Padding(
      padding: const EdgeInsets.only(top: Space.l, bottom: Space.s),
      child: Text(
        text,
        style: ts(
          fontSize: 12,
          fontWeight: FontWeight.w600,
          color: p.textMuted,
        ),
      ),
    );
    Widget hint(String text) => Padding(
      padding: const EdgeInsets.only(top: 6),
      child: Text(text, style: ts(fontSize: 12, color: p.textMuted)),
    );
    final quality = qualities.firstWhere(
      (q) => q.$1 == c.quality,
      orElse: () => qualities[1],
    );
    final lighting = lightings.firstWhere(
      (x) => x.$1 == s.light,
      orElse: () => lightings.first,
    );
    // A size set elsewhere (by an agent) is offered too.
    final sizes = [
      ...photoSizes,
      if (!photoSizes.contains((s.width, s.height))) (s.width, s.height),
    ];
    // The lens on a log scale: as many steps from 18 to 35 mm as from 100
    // to 200 mm.
    final focalSpan = math.log(_focalMax / _focalMin);
    // The place's size on a log scale too.
    final placeSpan = math.log(_placeMax / _placeMin);
    final placeAt =
        math.log(s.placeSizeM.clamp(_placeMin, _placeMax) / _placeMin) /
        placeSpan;
    final focalAt =
        math.log(s.focalMm.clamp(_focalMin, _focalMax) / _focalMin) / focalSpan;
    // A Material, for the ink of the folded section's tile.
    return Material(
      color: p.surface,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        side: BorderSide(color: p.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: SizedBox(
        width: 320,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.l, Space.s, Space.s, 0),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      l.photoTitle,
                      style: ts(fontSize: 14, fontWeight: FontWeight.w600),
                    ),
                  ),
                  Tooltip(
                    message: l.photoSceneCloseTip,
                    child: IconButton(
                      key: const ValueKey('sceneClose'),
                      onPressed: widget.onClose,
                      icon: Icon(
                        Icons.close_rounded,
                        size: 19,
                        color: p.textMuted,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            Flexible(
              child: SingleChildScrollView(
                padding: const EdgeInsets.fromLTRB(
                  Space.l,
                  0,
                  Space.l,
                  Space.l,
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text(
                      l.photoIntro,
                      style: ts(fontSize: 12.5, color: p.text),
                    ),
                    label(l.photoLighting),
                    Wrap(
                      key: const ValueKey('photoLighting'),
                      spacing: 6,
                      runSpacing: 6,
                      children: [
                        for (final (id, name, _) in lightings)
                          ChoiceChip(
                            label: Text(name),
                            selected: s.light == id,
                            onSelected: (_) => _light(id),
                          ),
                      ],
                    ),
                    hint(lighting.$3),
                    if (s.light == 'environment') ...[
                      const SizedBox(height: Space.s),
                      Row(
                        children: [
                          Expanded(
                            child: DropdownButton<String>(
                              key: const ValueKey('photoEnvironment'),
                              isExpanded: true,
                              value: _environments.contains(s.environment)
                                  ? s.environment
                                  : null,
                              items: [
                                for (final id in _environments)
                                  DropdownMenuItem(
                                    value: id,
                                    child: Text(_environmentName(id)),
                                  ),
                              ],
                              onChanged: (id) {
                                if (id != null) {
                                  _scene(_with(s, environment: id));
                                }
                              },
                            ),
                          ),
                          const SizedBox(width: Space.s),
                          TextButton(
                            onPressed: _addEnvironment,
                            child: Text(l.photoEnvironmentAdd),
                          ),
                        ],
                      ),
                    ],
                    const SizedBox(height: Space.s),
                    _SliderRow(
                      key: const ValueKey('photoTurn'),
                      label: l.photoTurn,
                      value: s.turnDeg,
                      min: -180,
                      max: 180,
                      divisions: 72,
                      text: '${s.turnDeg.round()}°',
                      onChanged: (v) => _scene(_with(s, turnDeg: v)),
                    ),
                    _SliderRow(
                      key: const ValueKey('photoBrightness'),
                      label: l.photoBrightness,
                      value: s.brightness,
                      min: 0.25,
                      max: 4,
                      divisions: 15,
                      text: '× ${context.number(s.brightness, 2)}',
                      onChanged: (v) => _scene(_with(s, brightness: v)),
                    ),
                    if (s.light == 'environment')
                      _SliderRow(
                        key: const ValueKey('photoPlaceSize'),
                        label: l.photoPlaceSize,
                        value: placeAt,
                        min: 0,
                        max: 1,
                        divisions: 100,
                        text: '${s.placeSizeM.round()} m',
                        onChanged: (t) => _scene(
                          _with(
                            s,
                            placeSizeM: (_placeMin * math.exp(t * placeSpan))
                                .roundToDouble(),
                          ),
                        ),
                      ),
                    _SliderRow(
                      key: const ValueKey('photoFocal'),
                      label: l.photoFocal,
                      value: focalAt,
                      min: 0,
                      max: 1,
                      divisions: 100,
                      text: '${s.focalMm.round()} mm',
                      onChanged: (t) => _scene(
                        _with(
                          s,
                          focalMm: (_focalMin * math.exp(t * focalSpan))
                              .roundToDouble(),
                        ),
                      ),
                    ),
                    hint(l.photoSceneHint),
                    label(l.photoSize),
                    DropdownButton<(int, int)>(
                      key: const ValueKey('photoSize'),
                      isExpanded: true,
                      value: (s.width, s.height),
                      items: [
                        for (final (w, h) in sizes)
                          DropdownMenuItem(
                            value: (w, h),
                            child: Text(w == 0 ? l.photoSizeView : '$w × $h'),
                          ),
                      ],
                      onChanged: (size) {
                        if (size != null) _scene(_with(s, size: size));
                      },
                    ),
                    label(l.photoQuality),
                    SegmentedButton<String>(
                      key: const ValueKey('photoQuality'),
                      segments: [
                        for (final (id, name, _, _) in qualities)
                          ButtonSegment(value: id, label: Text(name)),
                      ],
                      selected: {quality.$1},
                      onSelectionChanged: (q) => _choice(quality: q.first),
                    ),
                    hint(quality.$3),
                    Theme(
                      // No divider lines around the folded section.
                      data: Theme.of(context)
                          .copyWith(dividerColor: Colors.transparent),
                      child: ExpansionTile(
                        tilePadding: EdgeInsets.zero,
                        childrenPadding: EdgeInsets.zero,
                        title: Text(l.photoAdvanced, style: ts(fontSize: 13)),
                        children: [
                          _CountRow(
                            label: l.photoSamples,
                            controller: _samples,
                            placeholder: quality.$4,
                            onChanged: (_) => _choice(),
                          ),
                          _CountRow(
                            label: l.photoBounces,
                            controller: _bounces,
                            placeholder: switch (quality.$1) {
                              'quick' => '4',
                              'good' => '6',
                              _ => '10',
                            },
                            onChanged: (_) => _choice(),
                          ),
                          SwitchListTile(
                            contentPadding: EdgeInsets.zero,
                            dense: true,
                            title: Text(l.photoDenoise),
                            value: c.denoise,
                            onChanged: (v) => _choice(denoise: v),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(height: Space.s),
                    FilledButton.icon(
                      key: const ValueKey('photoRender'),
                      onPressed: widget.onShoot,
                      icon: const Icon(Icons.photo_camera_outlined, size: 18),
                      label: Text(l.photoRender),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// A labelled slider with its value written beside it.
class _SliderRow extends StatelessWidget {
  const _SliderRow({
    super.key,
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.divisions,
    required this.text,
    required this.onChanged,
  });

  final String label;
  final double value;
  final double min;
  final double max;
  final int divisions;
  final String text;
  final ValueChanged<double> onChanged;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Row(
      children: [
        SizedBox(
          width: 84,
          child: Text(label, style: ts(fontSize: 13, color: p.text)),
        ),
        Expanded(
          child: Slider(
            value: value.clamp(min, max),
            min: min,
            max: max,
            divisions: divisions,
            onChanged: onChanged,
          ),
        ),
        SizedBox(
          width: 52,
          child: Text(
            text,
            textAlign: TextAlign.right,
            style: ts(fontSize: 13, color: p.text, tabular: true),
          ),
        ),
      ],
    );
  }
}

/// A whole number typed in the advanced settings; empty keeps the
/// quality's own (shown as the placeholder).
class _CountRow extends StatelessWidget {
  const _CountRow({
    required this.label,
    required this.controller,
    required this.placeholder,
    required this.onChanged,
  });

  final String label;
  final TextEditingController controller;
  final String placeholder;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Text(label, style: ts(fontSize: 13, color: p.text)),
          ),
          SizedBox(
            width: 96,
            child: TextField(
              controller: controller,
              onChanged: onChanged,
              textAlign: TextAlign.right,
              keyboardType: TextInputType.number,
              inputFormatters: [FilteringTextInputFormatter.digitsOnly],
              style: ts(fontSize: 13, color: p.text, tabular: true),
              decoration: InputDecoration(
                isDense: true,
                hintText: placeholder,
                contentPadding: const EdgeInsets.symmetric(
                  horizontal: 8,
                  vertical: 7,
                ),
                border: const OutlineInputBorder(),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Progress of the photo over the 3D view, with Stop, Save and Close.
class PhotoBar extends StatelessWidget {
  const PhotoBar({
    super.key,
    required this.status,
    required this.onStop,
    required this.onSave,
    required this.onClose,
  });

  final PhotoStatus status;
  final VoidCallback onStop;
  final VoidCallback onSave;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    final p = Tokens.of(context);
    final s = status;
    final seconds = context.number(s.seconds, s.seconds < 10 ? 1 : 0);
    final error = s.error;
    final text = error != null
        ? l.photoFailed(error)
        : !s.finished
        ? l.photoRendering(s.samples, s.total, seconds)
        : s.samples >= s.total
        ? l.photoDone(seconds)
        : l.photoStopped(s.samples);
    return Container(
      padding: const EdgeInsets.fromLTRB(Space.l, Space.s, Space.s, Space.s),
      decoration: BoxDecoration(
        color: p.surface.withValues(alpha: 0.92),
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        border: Border.all(color: p.border),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  text,
                  key: const ValueKey('photoProgress'),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: ts(
                    fontSize: 12.5,
                    color: error != null ? p.danger : p.text,
                    tabular: true,
                  ),
                ),
                const SizedBox(height: 6),
                LinearProgressIndicator(
                  value: s.total == 0 ? 0 : s.samples / s.total,
                  minHeight: 3,
                  borderRadius: BorderRadius.circular(2),
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.m),
          if (!s.finished)
            Tooltip(
              message: l.photoStopTip,
              child: TextButton(onPressed: onStop, child: Text(l.photoStop)),
            ),
          FilledButton.tonal(
            key: const ValueKey('photoSave'),
            onPressed: s.samples > 0 && error == null ? onSave : null,
            child: Text(l.photoSave),
          ),
          const SizedBox(width: 4),
          Tooltip(
            message: l.photoCloseTip,
            child: IconButton(
              key: const ValueKey('photoClose'),
              onPressed: onClose,
              icon: Icon(Icons.close_rounded, size: 19, color: p.textMuted),
            ),
          ),
        ],
      ),
    );
  }
}
