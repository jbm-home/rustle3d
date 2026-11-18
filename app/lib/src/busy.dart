// Feedback on long steps (docs/ux.md §2): a step that takes seconds runs off
// the UI thread, and a card says what is being done, with a bar when its
// length is known and a Stop button where stopping is safe.

import 'dart:async';

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'rust/api/engine.dart';

/// Shown only for steps longer than this: quick ones do not flash a card.
const _showAfter = Duration(milliseconds: 300);

/// How often the card asks the engine where the step is.
const _pollEvery = Duration(milliseconds: 100);

/// Run [job] (a call into the engine that may take seconds) and return its
/// result; past a moment, a modal card titled [title] shows its progress.
/// With [cancellable], the card has a Stop button: the step then fails
/// with the `cancelled` code and changes nothing.
Future<T> runBusy<T>(
  BuildContext context,
  Engine engine,
  String title,
  Future<T> Function() job, {
  bool cancellable = false,
}) async {
  OverlayEntry? card;
  final overlay = Overlay.of(context, rootOverlay: true);
  final timer = Timer(_showAfter, () {
    card = OverlayEntry(
      builder: (_) =>
          _BusyCard(engine: engine, title: title, cancellable: cancellable),
    );
    overlay.insert(card!);
  });
  try {
    return await job();
  } finally {
    timer.cancel();
    card?.remove();
  }
}

class _BusyCard extends StatefulWidget {
  const _BusyCard({
    required this.engine,
    required this.title,
    required this.cancellable,
  });

  final Engine engine;
  final String title;
  final bool cancellable;

  @override
  State<_BusyCard> createState() => _BusyCardState();
}

class _BusyCardState extends State<_BusyCard> {
  StepProgress? _progress;
  bool _stopping = false;
  late final Timer _poll;

  @override
  void initState() {
    super.initState();
    _read();
    _poll = Timer.periodic(_pollEvery, (_) => _read());
  }

  void _read() {
    try {
      final now = widget.engine.progress();
      if (mounted) setState(() => _progress = now);
    } on ApiError {
      // The step reports its own errors when it ends.
    }
  }

  @override
  void dispose() {
    _poll.cancel();
    super.dispose();
  }

  String _what(BuildContext context, StepProgress? p) {
    final l = context.l;
    return switch (p?.task) {
      'template.sizes' when p!.done == 0 => l.busySizesStart(p.total),
      'template.sizes' => l.busySizes(p!.item, p.done, p.total),
      'body.fit' => l.busyBodyFit,
      'drape.prepare' => l.busyDrape,
      'export' => l.busyExport,
      'import' => l.busyImport,
      _ => l.busyStep,
    };
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final progress = _progress;
    final total = progress?.total ?? 0;
    final value = total > 0 ? progress!.done / total : null;
    return Stack(
      children: [
        ModalBarrier(
          dismissible: false,
          color: Colors.black.withValues(alpha: 0.18),
        ),
        Center(
          child: Material(
            color: p.surface,
            elevation: 8,
            borderRadius: BorderRadius.circular(Tokens.radius(context, 1)),
            child: SizedBox(
              width: 360,
              child: Padding(
                padding: const EdgeInsets.all(Space.l),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      widget.title,
                      style: ts(
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                        color: p.text,
                      ),
                    ),
                    const SizedBox(height: Space.m),
                    LinearProgressIndicator(
                      value: value,
                      color: p.accent,
                      backgroundColor: p.accentSoft,
                      minHeight: 6,
                      borderRadius: BorderRadius.circular(3),
                    ),
                    const SizedBox(height: Space.s),
                    Text(
                      _what(context, progress),
                      style: ts(fontSize: 13, color: p.textMuted),
                    ),
                    // Fitting a body or placing the pieces cannot stop
                    // halfway: no Stop then.
                    if (widget.cancellable &&
                        !const {
                          'body.fit',
                          'drape.prepare',
                        }.contains(progress?.task)) ...[
                      const SizedBox(height: Space.m),
                      Align(
                        alignment: Alignment.centerRight,
                        child: TextButton(
                          onPressed: _stopping
                              ? null
                              : () {
                                  setState(() => _stopping = true);
                                  widget.engine.cancel();
                                },
                          child: Text(context.l.busyStop),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}
