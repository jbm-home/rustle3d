// The guided flow for a garment (docs/specs/11, docs/ux.md §1): a step bar
// Measurements → Pattern → Sew → Fabric → Drape. Each step has a one-line
// goal, a main action and the tool it uses. Steps can be taken in any
// order, and the guide can be closed at any time.

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';

enum GuideStep {
  measurements(null),
  pattern('select'),
  sew('sew'),
  fabric(null),
  drape(null);

  const GuideStep(this.tool);

  /// The tool rail entry the step uses.
  final String? tool;

  String label(L l) => switch (this) {
    measurements => l.stepMeasurements,
    pattern => l.stepPattern,
    sew => l.stepSew,
    fabric => l.stepFabric,
    drape => l.stepDrape,
  };

  String goal(L l) => switch (this) {
    measurements => l.stepMeasurementsGoal,
    pattern => l.stepPatternGoal,
    sew => l.stepSewGoal,
    fabric => l.stepFabricGoal,
    drape => l.stepDrapeGoal,
  };

  /// The banner's main button, if the step has one.
  String? action(L l) => switch (this) {
    measurements => l.stepMeasurementsAction,
    pattern => l.stepPatternAction,
    sew => l.stepSewAction,
    fabric => null,
    drape => l.simulate,
  };

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
    final l = context.l;
    final action = step == GuideStep.drape && running
        ? l.pause
        : step.action(l);
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
                        l.stepOf(step.index + 1, GuideStep.values.length),
                        style: ts(
                          fontSize: 12.5,
                          fontWeight: FontWeight.w600,
                          color: p.accent,
                        ),
                      ),
                      const SizedBox(width: Space.m),
                      Expanded(
                        child: Text(
                          step.goal(l),
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
                          child: Text(l.guideNext),
                        ),
                      ],
                      IconButton(
                        tooltip: l.guideHide,
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
