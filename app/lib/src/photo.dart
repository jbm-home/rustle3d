// Photos: the dialog that chooses how to render one, and the bar over the
// 3D view while it refines (docs/specs/14-photoreal-rendering.md).

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
  });

  /// `quick`, `good` or `best`.
  final String quality;

  /// `studio`, `soft` or `dramatic`.
  final String lighting;

  /// Index into [photoSizes].
  final int size;

  /// Instead of the quality's (advanced).
  final int? samples;
  final int? bounces;
  final bool denoise;

  PhotoRequest get request {
    final (width, height) = photoSizes[size.clamp(0, photoSizes.length - 1)];
    return PhotoRequest(
      quality: quality,
      lighting: lighting,
      width: width,
      height: height,
      samples: samples,
      bounces: bounces,
      denoise: denoise,
    );
  }
}

/// Ask how to render a photo; `null` when cancelled.
Future<PhotoChoice?> askPhoto(BuildContext context, PhotoChoice initial) {
  return showDialog<PhotoChoice>(
    context: context,
    builder: (_) => _PhotoDialog(initial: initial),
  );
}

class _PhotoDialog extends StatefulWidget {
  const _PhotoDialog({required this.initial});

  final PhotoChoice initial;

  @override
  State<_PhotoDialog> createState() => _PhotoDialogState();
}

class _PhotoDialogState extends State<_PhotoDialog> {
  late String _quality = widget.initial.quality;
  late String _lighting = widget.initial.lighting;
  late int _size = widget.initial.size;
  late bool _denoise = widget.initial.denoise;
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
