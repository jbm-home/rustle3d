// Photos: the dialog that chooses how to render one, and the bar over the
// 3D view while it refines (docs/specs/14-photoreal-rendering.md).

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

/// How to render a photo, as last chosen.
class PhotoChoice {
  const PhotoChoice({
    this.quality = 'good',
    this.lighting = 'studio',
    this.size = 0,
    this.samples,
    this.bounces,
    this.denoise = true,
    this.environment = 'ferndale',
    this.turnDeg = 0,
    this.brightness = 1,
  });

  /// `quick`, `good` or `best`.
  final String quality;

  /// `studio`, `soft`, `dramatic`, or `environment` (lit by
  /// [environment], spec 35).
  final String lighting;

  /// A built-in environment's name or a `.hdr` file in the project.
  final String environment;

  /// The environment's turn (degrees) and brightness factor.
  final double turnDeg;
  final double brightness;

  /// Index into [photoSizes].
  final int size;

  /// Instead of the quality's (advanced).
  final int? samples;
  final int? bounces;
  final bool denoise;

  PhotoRequest get request {
    final (width, height) = photoSizes[size.clamp(0, photoSizes.length - 1)];
    final lit = lighting == 'environment';
    return PhotoRequest(
      quality: quality,
      lighting: lit ? 'studio' : lighting,
      width: width,
      height: height,
      samples: samples,
      bounces: bounces,
      denoise: denoise,
      environment: lit ? environment : null,
      turnDeg: turnDeg,
      brightness: brightness,
    );
  }
}

/// Ask how to render a photo; `null` when cancelled.
Future<PhotoChoice?> askPhoto(
  BuildContext context,
  Engine engine,
  PhotoChoice initial,
) {
  return showDialog<PhotoChoice>(
    context: context,
    builder: (_) => _PhotoDialog(engine: engine, initial: initial),
  );
}

class _PhotoDialog extends StatefulWidget {
  const _PhotoDialog({required this.engine, required this.initial});

  final Engine engine;
  final PhotoChoice initial;

  @override
  State<_PhotoDialog> createState() => _PhotoDialogState();
}

class _PhotoDialogState extends State<_PhotoDialog> {
  late String _quality = widget.initial.quality;
  late String _lighting = widget.initial.lighting;
  late int _size = widget.initial.size;
  late bool _denoise = widget.initial.denoise;
  late String _environment = widget.initial.environment;
  late double _turn = widget.initial.turnDeg;
  late double _brightness = widget.initial.brightness;
  late List<String> _environments = _listEnvironments();
  late final _samples = TextEditingController(
    text: widget.initial.samples?.toString() ?? '',
  );
  late final _bounces = TextEditingController(
    text: widget.initial.bounces?.toString() ?? '',
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
      setState(() {
        _environments = _listEnvironments();
        _environment = path;
      });
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

  void _render() {
    Navigator.pop(
      context,
      PhotoChoice(
        quality: _quality,
        lighting: _lighting,
        size: _size,
        samples: int.tryParse(_samples.text),
        bounces: int.tryParse(_bounces.text),
        denoise: _denoise,
        environment: _environment,
        turnDeg: _turn,
        brightness: _brightness,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    final p = Tokens.of(context);
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
    final quality = qualities.firstWhere((q) => q.$1 == _quality);
    final lighting = lightings.firstWhere((x) => x.$1 == _lighting);
    return AlertDialog(
      title: Text(l.photoTitle),
      content: SizedBox(
        width: 440,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(l.photoIntro, style: ts(fontSize: 13, color: p.text)),
              label(l.photoQuality),
              SegmentedButton<String>(
                key: const ValueKey('photoQuality'),
                segments: [
                  for (final (id, name, _, _) in qualities)
                    ButtonSegment(value: id, label: Text(name)),
                ],
                selected: {_quality},
                onSelectionChanged: (s) => setState(() => _quality = s.first),
              ),
              hint(quality.$3),
              label(l.photoLighting),
              SegmentedButton<String>(
                key: const ValueKey('photoLighting'),
                segments: [
                  for (final (id, name, _) in lightings)
                    ButtonSegment(value: id, label: Text(name)),
                ],
                selected: {_lighting},
                onSelectionChanged: (s) => setState(() => _lighting = s.first),
              ),
              hint(lighting.$3),
              if (_lighting == 'environment') ...[
                const SizedBox(height: Space.s),
                Row(
                  children: [
                    Expanded(
                      child: DropdownButton<String>(
                        key: const ValueKey('photoEnvironment'),
                        isExpanded: true,
                        value: _environments.contains(_environment)
                            ? _environment
                            : null,
                        items: [
                          for (final id in _environments)
                            DropdownMenuItem(
                              value: id,
                              child: Text(_environmentName(id)),
                            ),
                        ],
                        onChanged: (id) =>
                            setState(() => _environment = id ?? _environment),
                      ),
                    ),
                    const SizedBox(width: Space.s),
                    TextButton(
                      onPressed: _addEnvironment,
                      child: Text(l.photoEnvironmentAdd),
                    ),
                  ],
                ),
                _SliderRow(
                  label: l.photoTurn,
                  value: _turn,
                  min: -180,
                  max: 180,
                  divisions: 72,
                  text: '${_turn.round()}°',
                  onChanged: (v) => setState(() => _turn = v),
                ),
                _SliderRow(
                  label: l.photoBrightness,
                  value: _brightness,
                  min: 0.25,
                  max: 4,
                  divisions: 15,
                  text: '× ${context.number(_brightness, 2)}',
                  onChanged: (v) => setState(() => _brightness = v),
                ),
              ],
              label(l.photoSize),
              DropdownButton<int>(
                isExpanded: true,
                value: _size,
                items: [
                  for (final (i, (w, h)) in photoSizes.indexed)
                    DropdownMenuItem(
                      value: i,
                      child: Text(i == 0 ? l.photoSizeView : '$w × $h'),
                    ),
                ],
                onChanged: (i) => setState(() => _size = i ?? 0),
              ),
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
                    ),
                    _CountRow(
                      label: l.photoBounces,
                      controller: _bounces,
                      placeholder: switch (_quality) {
                        'quick' => '4',
                        'good' => '6',
                        _ => '10',
                      },
                    ),
                    SwitchListTile(
                      contentPadding: EdgeInsets.zero,
                      dense: true,
                      title: Text(l.photoDenoise),
                      value: _denoise,
                      onChanged: (v) => setState(() => _denoise = v),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton.icon(
          key: const ValueKey('photoRender'),
          onPressed: _render,
          icon: const Icon(Icons.photo_camera_outlined, size: 18),
          label: Text(l.photoRender),
        ),
      ],
    );
  }
}

/// A labelled slider with its value written beside it.
class _SliderRow extends StatelessWidget {
  const _SliderRow({
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
          width: 96,
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
  });

  final String label;
  final TextEditingController controller;
  final String placeholder;

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
