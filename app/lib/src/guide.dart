// The guided flow for a garment (docs/specs/11, docs/ux.md §1): a step bar
// Measurements → Pattern → Sew → Fabric → Drape. Each step has a one-line
// goal, a main action and the tool it uses. Steps can be taken in any
// order, and the guide can be closed at any time.

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';

enum GuideStep {
  measurements(
    'Measurements',
    'Check the body the garment is made for: its size and measurements.',
    'Open Body',
    null,
  ),
  pattern(
    'Pattern',
    'Shape the pieces: drag their points on the pattern, or ⇧-drag a hem or a seam on the garment.',
    'Add a piece',
    'select',
  ),
  sew(
    'Sew',
    'Join the pieces: click an edge on the pattern, then the edge to sew it to.',
    'Sew tool',
    'sew',
  ),
  fabric(
    'Fabric',
    'Choose what the garment is made of, in the panel on the right.',
    null,
    null,
  ),
  drape(
    'Drape',
    'Watch it drape on the body. Then drag the cloth, or ⌥-click to pin it.',
    'Simulate',
    null,
  );

  const GuideStep(this.label, this.goal, this.action, this.tool);

  final String label;
  final String goal;

  /// The banner's main button, if the step has one.
  final String? action;

  /// The tool rail entry the step uses.
  final String? tool;

  GuideStep? get next => index + 1 < values.length ? values[index + 1] : null;

  /// Where a project picks up: a sewn garment is ready to drape, pieces
  /// still need sewing, an empty pattern needs pieces.
  static GuideStep firstFor({required int panels, required int seams}) {
    if (seams > 0) return GuideStep.drape;
    if (panels > 0) return GuideStep.sew;
    return GuideStep.pattern;
  }
}

/// The current step's goal and actions, under the top bar.
class GuideBanner extends StatelessWidget {
  const GuideBanner({
    super.key,
    required this.step,
    required this.visible,
    required this.running,
    required this.onAction,
    required this.onNext,
    required this.onClose,
  });

  final GuideStep step;
  final bool visible;

  /// The simulation runs (the Drape action then pauses it).
  final bool running;
  final VoidCallback onAction;
  final VoidCallback? onNext;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final action = step == GuideStep.drape && running ? 'Pause' : step.action;
    return AnimatedSize(
      duration: Motion.normal,
      curve: Motion.curve,
      child: !visible
          ? const SizedBox(width: double.infinity)
          : Padding(
              padding: const EdgeInsets.fromLTRB(
                Space.s + 60,
                0,
                Space.m,
                Space.s,
              ),
              child: AnimatedSwitcher(
                duration: Motion.normal,
                child: Container(
                  key: ValueKey(step),
                  padding: const EdgeInsets.symmetric(
                    horizontal: Space.l,
                    vertical: Space.s,
                  ),
                  decoration: BoxDecoration(
                    color: p.accentSoft,
                    borderRadius: BorderRadius.circular(kRadius * 0.8),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.flag_rounded, size: 16, color: p.accent),
                      const SizedBox(width: Space.s),
                      Text(
                        'Step ${step.index + 1} of ${GuideStep.values.length}',
                        style: ts(
                          fontSize: 12.5,
                          fontWeight: FontWeight.w600,
                          color: p.accent,
                        ),
                      ),
                      const SizedBox(width: Space.m),
                      Expanded(
                        child: Text(
                          step.goal,
                          style: ts(fontSize: 13, color: p.text),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (action != null) ...[
                        const SizedBox(width: Space.m),
                        FilledButton(
                          onPressed: onAction,
                          style: FilledButton.styleFrom(
                            visualDensity: VisualDensity.compact,
                          ),
                          child: Text(action),
                        ),
                      ],
                      if (onNext != null) ...[
                        const SizedBox(width: Space.s),
                        TextButton(
                          onPressed: onNext,
                          style: TextButton.styleFrom(
                            visualDensity: VisualDensity.compact,
                          ),
                          child: const Text('Next  →'),
                        ),
                      ],
                      IconButton(
                        tooltip:
                            'Hide the guide (click a step to bring it back)',
                        onPressed: onClose,
                        visualDensity: VisualDensity.compact,
                        icon: Icon(
                          Icons.close_rounded,
                          size: 16,
                          color: p.textMuted,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
    );
  }
}
